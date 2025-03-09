use bsky_sdk::api::app::bsky::feed::post;
use bsky_sdk::api::types::string::Datetime;
use bsky_sdk::BskyAgent;

use glob::glob;
use grep::{matcher::Matcher, regex, searcher::sinks};
use rand::random_range;
use rand::seq::SliceRandom;

use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use redis::{aio::{self, MultiplexedConnection}, AsyncCommands, Client};

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

async fn reshuffle_quotes(filter: &QuoteFilter, mut con: impl redis::aio::ConnectionLike + AsyncCommands, output_queue: &str) -> Result<(), ()> {
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
            pipeline.lpush(output_queue,file_contents.as_str());
        }
        let _: () = pipeline.query_async(&mut con).await.unwrap();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis = redis::Client::open(std::env::var("REDIS_URL").unwrap_or("redis://localhost".to_string()))?;
    let con = redis.get_multiplexed_async_connection().await?;

    let agent = BskyAgent::builder().build().await?;
    let _session = agent
        .login(
            std::env::var("BLUESKY_USERNAME").unwrap_or_default(),
            std::env::var("BLUESKY_PASSWORD").unwrap_or_default(),
        )
        .await;

    let sched = JobScheduler::new().await?;
    let agent = Arc::new(Mutex::new(agent));
    let filter = Arc::new(QuoteFilter {
        content: r"\b(?i:mother|mommy|mama|mom)\b".to_string(),
        path: "quotes/**/*.txt".to_string(),
        dates: vec![],
    });

    // Add async job
    sched
        .add(Job::new_async("0/10,5/10 * * * * *", move |_uuid, _| {
            let filter = filter.clone();
            let mut con = con.clone();
            let agent = agent.clone();

            Box::pin(async move {
                let _ = reshuffle_quotes(&filter, con.clone(), "test:queue").await.unwrap();
                let text: String = con.lpop("test:queue", None).await.unwrap();
                let post = prepare_post(text.as_str());
                let agent = agent.lock().await;
                if let Err(_) = agent.create_record(post).await {
                    println!("{}\n", text)
                }
            })
        })?)
        .await?;

    sched.start().await.unwrap();
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
