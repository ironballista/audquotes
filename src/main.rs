use bsky_sdk::BskyAgent;
use bsky_sdk::api::app::bsky::feed::post;
use bsky_sdk::api::types::string::Datetime;

use glob::glob;
use grep::{matcher::Matcher, regex, searcher::sinks};
use rand::random_range;
use rand::seq::SliceRandom;

use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use redis::{
    AsyncCommands, Client,
    aio::{self, MultiplexedConnection},
};

const DEFAULT_QUEUE: &str = "queue:default";
const EVENT_QUEUE: &str = "queue:event";

// See https://cron.help for what these strings mean
const POSTING_INTERVAL_CRON: &str = "00,30 * * * * * *"; 
const EVENT_UPDATE_INTERVAL: &str = "55 23 * * *";

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

struct QuoteFilter {
    path: String,
    content: String,
    dates: Vec<String>,
}

fn read_files(filter: &QuoteFilter) -> Vec<String> {
    let matcher = regex::RegexMatcher::new(&filter.content).unwrap();
    let mut searcher = grep::searcher::Searcher::new();
    let mut results = Vec::new();

    for file in glob(&filter.path).unwrap() {
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

async fn reshuffle_quotes(
    filter: &QuoteFilter,
    mut con: impl redis::aio::ConnectionLike + AsyncCommands,
    output_queue: &str,
) -> Result<(), ()> {
    let len: u64 = con.llen(output_queue).await.unwrap();
    // NOTE: The following assumes the queue hasn't been repopulated by any other client
    //       in-between the call to llen and the execution of the pipeline.
    //       Hopefully won't be a problem :)
    if len == 0 {
        let mut file_contents = read_files(filter);

        {
            let mut rand = rand::rng();
            file_contents.shuffle(&mut rand);
        }

        let mut pipeline = redis::pipe();
        for file_contents in file_contents.into_iter() {
            pipeline.lpush(output_queue, file_contents.as_str());
        }
        let _: () = pipeline.query_async(&mut con).await.unwrap();
    }

    Ok(())
}

async fn get_quote(
    filter: &QuoteFilter,
    mut con: impl redis::aio::ConnectionLike + AsyncCommands + Clone,
) -> Result<String, ()> {
    // 1: Attempt to read from the event (priority) queue
    let event_quote: Option<String> = con.lpop(EVENT_QUEUE, None).await.ok();
    if let Some(quote) = event_quote {
        return Ok(quote);
    }

    // 2: Otherwise, we read from the regular queue, repopulating it if it's empty
    reshuffle_quotes(filter, con.clone(), DEFAULT_QUEUE).await?;
    con.lpop(DEFAULT_QUEUE, None).await.map_err(|_| ())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis =
        redis::Client::open(std::env::var("REDIS_URL").unwrap_or("redis://localhost".to_string()))?;
    let con = redis.get_multiplexed_async_connection().await?;

    let agent = BskyAgent::builder().build().await?;
    let _session = agent
        .login(
            std::env::var("BLUESKY_USERNAME").unwrap_or_default(),
            std::env::var("BLUESKY_PASSWORD").unwrap_or_default(),
        )
        .await?;

    let sched = JobScheduler::new().await?;
    let agent = Arc::new(Mutex::new(agent));
    
    /*
        let event_filter = Arc::new(QuoteFilter {
            content: r"\b(?i:mother|mommy|mama|mom)\b".to_string(),
            path: "test/**/*.txt".to_string(),
            dates: vec![],
        });
    */

    let regular_filter = Arc::new(QuoteFilter {
        content: r".*".to_string(),
        path: "quotes/**/*.txt".to_string(),
        dates: vec![],
    });

    let (con_poster, con_event_monitor) = (con.clone(), con.clone());
    let (agent_poster, agent_event_monitor) = (agent.clone(), agent.clone());

    // Add async job
    sched
        .add(Job::new_async(POSTING_INTERVAL_CRON, move |_uuid, _| {
            let filter = regular_filter.clone();
            let con = con_poster.clone();
            let agent = agent_poster.clone();

            Box::pin(async move {
                let text: String = get_quote(&filter, con).await.unwrap();
                let post = prepare_post(text.as_str());
                let agent = agent.lock().await;
                if let Err(e) = agent.create_record(post).await {
                    eprintln!("Could not post quote: {e}")
                }
            })
        })?)
        .await?;

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

    sched.start().await.unwrap();
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
