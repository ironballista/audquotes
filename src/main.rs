use bsky_sdk::api::app::bsky::feed::post;
use bsky_sdk::api::types::string::Datetime;
use bsky_sdk::{BskyAgent, api::types::Object};

use glob::glob;
use grep::{regex, searcher::sinks};
use rand::seq::SliceRandom;
use redis::aio::ConnectionManagerConfig;

use std::{sync::Arc, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler};

use redis::AsyncCommands;

const DEFAULT_QUEUE: &str = "queue:default";
const EVENT_QUEUE: &str = "queue:event";

// See https://cron.help for what these strings mean
const POSTING_INTERVAL_CRON: &str = "0 0,30 * * * *";
const POSTING_INTERVAL_DEBUG: &str = "1/10 * * * * *";
const EVENT_UPDATE_INTERVAL: &str = "55 23 * * *";

const POSTING_RETRIES: i32 = 5;

fn prepare_post<I: Into<String>>(text: I) -> post::RecordData {
    post::RecordData {
        text: text.into(),
        created_at: Datetime::now(),
        embed: None,
        entities: None,
        facets: None,
        labels: None,
        langs: None,
        reply: None,
        tags: None,
    }
}

#[derive(Clone, Debug)]
struct QuoteFilter {
    path: String,
    content: String,
    dates: Vec<String>,
}

impl QuoteFilter {
    pub async fn get_quote(
        &self,
        mut con: impl redis::aio::ConnectionLike + AsyncCommands + Clone,
    ) -> Result<String, ()> {
        // 1: Attempt to read from the event (priority) queue
        let event_quote: Option<String> = con.lpop(EVENT_QUEUE, None).await.ok();
        if let Some(quote) = event_quote {
            return Ok(quote);
        }

        // 2: Otherwise, we read from the regular queue, repopulating it if it's empty
        self.reshuffle_quotes(con.clone(), DEFAULT_QUEUE).await?;
        con.lpop(DEFAULT_QUEUE, None).await.map_err(|_| ())
    }

    async fn reshuffle_quotes(
        &self,
        mut con: impl redis::aio::ConnectionLike + AsyncCommands,
        output_queue: &str,
    ) -> Result<(), ()> {
        let len: u64 = con.llen(output_queue).await.map_err(|_| ())?;
        // NOTE: The following assumes the queue hasn't been repopulated by any other client
        //       in-between the call to llen and the execution of the pipeline.
        //       Hopefully won't be a problem :)
        if len == 0 {
            let mut file_contents = self.read_files();

            {
                let mut rand = rand::rng();
                file_contents.shuffle(&mut rand);
            }

            let mut pipeline = redis::pipe();
            for file_contents in file_contents.into_iter() {
                pipeline.lpush(output_queue, file_contents.as_str());
            }
            let _: () = pipeline.query_async(&mut con).await.map_err(|_| ())?;
        }

        Ok(())
    }

    fn read_files(&self) -> Vec<String> {
        let matcher = regex::RegexMatcher::new(&self.content).unwrap();
        let mut searcher = grep::searcher::Searcher::new();
        let mut results = Vec::new();

        for file in glob(&self.path).unwrap() {
            let file = match file {
                Ok(file) => file,
                Err(_) => continue,
            };

            let mut matched = false;
            let sink = sinks::Lossy(|_lnum, _line| {
                matched = true;
                Ok(false)
            });

            let search_result = searcher.search_path(&matcher, &file, sink);
            if !matched || search_result.is_err() {
                continue;
            }

            let contents = std::fs::read_to_string(file).unwrap();
            results.push(contents.trim().to_string());
        }

        results
    }
}

#[derive(Clone)]
struct RedisState {
    con_manager: redis::aio::ConnectionManager,
}

impl RedisState {
    pub async fn new(url: String) -> Result<Self, ()> {
        let redis = redis::Client::open(url).map_err(|_| ())?;
        let config = ConnectionManagerConfig::new()
            .set_response_timeout(std::time::Duration::from_secs(10))
            .set_number_of_retries(3);
        let con_manager = redis::aio::ConnectionManager::new_with_config(redis, config)
            .await
            .map_err(|_| ())?;

        Ok(RedisState { con_manager })
    }

    pub async fn fetch_quote(&self, filter: &QuoteFilter) -> Result<String, ()> {
        loop {
            match filter.get_quote(self.con_manager.clone()).await {
                Ok(text) => return Ok(text),
                Err(_) => eprintln!("Error fetching quote from redis storage. Retrying..."),
            };
        }
    }
}

#[derive(Clone)]
struct BlueskyState {
    bsky_agent: BskyAgent,
    bsky_session: Object<bsky_sdk::api::com::atproto::server::create_session::OutputData>,
}

impl BlueskyState {
    pub async fn new_session(username: String, password: String) -> Result<Self, ()> {
        let agent = BskyAgent::builder().build().await.map_err(|_| ())?;
        let session = agent.login(username, password).await.map_err(|_| ())?;

        Ok(Self {
            bsky_agent: agent,
            bsky_session: session,
        })
    }

    pub async fn submit_post(self, post: String) -> Result<(), ()> {
        let post = prepare_post(post.as_str());

        for current_try in 0..POSTING_RETRIES {
            if let Err(e) = self.bsky_agent.create_record(post.clone()).await {
                eprintln!("Could not post quote: `{e}`");
                eprintln!("Attempting to refresh login...");

                if let Err(e) = self
                    .bsky_agent
                    .resume_session(self.bsky_session.clone())
                    .await
                {
                    eprintln!("Failed to resume sessions due to following error: {e}")
                }
            } else {
                if current_try > 0 {
                    eprintln!("Successfully posted quote on retry #{current_try}");
                }
                return Ok(());
            }
        }

        Err(())
    }
}

#[derive(Clone)]
struct State {
    redis: RedisState,
    bsky_session: Option<BlueskyState>,
}

impl State {
    pub fn redis(&self) -> &RedisState {
        &self.redis
    }

    pub fn bsky(&self) -> Option<&BlueskyState> {
        self.bsky_session.as_ref()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let debug_mode = std::env::var("DEBUG").unwrap_or("0".to_string()) == "1";
    let use_bsky = std::env::var("USE_BLUESKY").unwrap_or("0".to_string()) == "1";

    let redis_state =
        RedisState::new(std::env::var("REDIS_URL").unwrap_or("redis://localhost".to_string()))
            .await
            .expect("Initial redis connection failure");
    let bsky_state = if use_bsky {
        Some(
            BlueskyState::new_session(
                std::env::var("BLUESKY_USERNAME").expect("Bluesky username not supplied"),
                std::env::var("BLUESKY_PASSWORD")
                    .expect("Bluesky application password not supplied"),
            )
            .await
            .expect("Could not connect to Bluesky with supplied credentials"),
        )
    } else {
        None
    };

    let app_state = Arc::new(State {
        redis: redis_state,
        bsky_session: bsky_state,
    });

    let sched = JobScheduler::new().await?;

    /*
    let event_filter = Arc::new(QuoteFilter {
        content: r"\b(?i:mother|mommy|mama|mom)\b".to_string(),
        path: "test/**/
*.txt".to_string(),
            dates: vec![],
        });
    */

    let regular_filter = Arc::new(QuoteFilter {
        content: r".*".to_string(),
        path: if !debug_mode {
            "quotes/**/*.txt".to_string()
        } else {
            "test/**/*.txt".to_string()
        },
        dates: vec![],
    });

    let posting_interval = if !debug_mode {
        POSTING_INTERVAL_CRON
    } else {
        POSTING_INTERVAL_DEBUG
    };

    let post_job = Job::new_async(posting_interval, move |_uuid, _| {
        let filter = regular_filter.clone();
        let app_state = app_state.clone();

        Box::pin(async move {
            // We try fetching a new quote from our redis storage until we succeed
            let text = match app_state.redis().fetch_quote(&filter).await {
                Ok(text) => text,
                Err(_) => {
                    eprintln!("Error fetching quote from redis storage.");
                    return;
                }
            };

            if let Some(bsky) = app_state.bsky() {
                if let Err(_) = bsky.clone().submit_post(text).await {
                    eprintln!("Error posting to bluesky.");
                    return;
                }
            } else {
                // Let's just print the quote!
                println!("{}\n", text);
            }
        })
    })?;

    // Add async job
    sched.add(post_job).await?;

    // sched
    //     .add(Job::new_async(EVENT_UPDATE_INTERVAL, move |_uuid, _| {
    //         let filter = event_filter.clone();
    //         let con = con_event_monitor.clone();
    //         let _agent = agent_event_monitor.clone(); // Can be used later to e.g. update profile

    //         Box::pin(async move {
    //             // For testing purposes, let's always upload events
    //             reshuffle_quotes(&filter, con.clone(), EVENT_QUEUE)
    //                 .await
    //                 .unwrap();
    //         })
    //     })?)
    //     .await?;

    sched
        .start()
        .await
        .expect("Error starting tokio scheduler. Shutting down...");
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
