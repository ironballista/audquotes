use bsky_sdk::api::app::bsky::feed::post;
use bsky_sdk::api::types::string::Datetime;
use bsky_sdk::BskyAgent;

use glob::glob;
use grep::{matcher::Matcher, regex, searcher::sinks};
use rand::random_range;

use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

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

fn read_files(filter: QuoteFilter) -> Vec<String> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_contents = Arc::new(read_files(QuoteFilter {
        content: r"\b(?i:mother|mommy|mama|mom)\b".to_string(),
        path: "quotes/**/*.txt".to_string(),
        dates: vec![],
    }));

    let agent = BskyAgent::builder().build().await?;
    let _session = agent
        .login(
            std::env::var("BLUESKY_USERNAME").unwrap_or_default(),
            std::env::var("BLUESKY_PASSWORD").unwrap_or_default(),
        )
        .await;

    let sched = JobScheduler::new().await?;
    let agent = Arc::new(Mutex::new(agent));

    // Add async job
    sched
        .add(Job::new_async("0/10,5/10 * * * * *", move |_uuid, _| {
            let file_contents = file_contents.clone();
            let agent = agent.clone();

            Box::pin(async move {
                let text  = file_contents[random_range(..file_contents.len())].as_str();
                let post = prepare_post(text);
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
