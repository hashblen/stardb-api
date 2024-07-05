use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::Path,
    time::{Duration, Instant},
};

mod achievement_series;
mod achievements;
mod avatars;
mod buddys;
mod texts;
mod w_engines;

use actix_web::rt::{self, Runtime};
use async_process::Command;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
struct AchieveSecondClass {
    #[serde(rename = "INMLOGABFMC")]
    id: i32,
    #[serde(rename = "FCNPHLPMPDJ")]
    name: String,
    #[serde(rename = "MDBIECECHIC")]
    priority: i32,
}

#[derive(serde::Deserialize)]
struct Achievement {
    #[serde(rename = "EKACDJHACBC")]
    id: i32,
    #[serde(rename = "IGEKGHFILLC")]
    series: i32,
    #[serde(rename = "LPMMOCBDALC")]
    name: String,
    #[serde(rename = "DJLPMAOEAJL")]
    description: String,
    #[serde(rename = "IGFHDCPGNIF")]
    reward: i32,
    #[serde(rename = "OMPENGGFEFF")]
    hidden: i32,
    #[serde(rename = "LDJMOLPOPOF")]
    priority: i32,
}

#[derive(serde::Deserialize)]
struct Rewards {
    #[serde(rename = "ILIBEPMJIAD")]
    id: i32,
    #[serde(rename = "HAOPBOILJMB")]
    rewards: Vec<Reward>,
}

#[derive(serde::Deserialize)]
struct Reward {
    #[serde(rename = "NOJCFGOCGBI")]
    id: i32,
    #[serde(rename = "CCNFEHBGKOC")]
    amount: i32,
}

#[derive(serde::Deserialize)]
struct Item {
    #[serde(rename = "HBKDOIKGNDE")]
    id: i32,
    #[serde(rename = "DIIDBBGLDOL")]
    name: String,
    #[serde(rename = "HAOPJCCHCGF")]
    rarity: i32,
}

#[derive(serde::Deserialize)]
struct Avatar {
    #[serde(rename = "HBKDOIKGNDE")]
    id: i32,
    #[serde(rename = "DIIDBBGLDOL")]
    name: String,
}

#[derive(serde::Deserialize)]
struct Weapon {
    #[serde(rename = "NOJCFGOCGBI")]
    id: i32,
}

#[derive(serde::Deserialize)]
struct Buddy {
    #[serde(rename = "HBKDOIKGNDE")]
    id: i32,
}

struct Configs {
    achievement_second_class_config: HashMap<String, Vec<AchieveSecondClass>>,
    achievement: HashMap<String, Vec<Achievement>>,
    once_reward: HashMap<String, Vec<Rewards>>,
    item: HashMap<String, Vec<Item>>,
    avatar: HashMap<String, Vec<Avatar>>,
    weapon: HashMap<String, Vec<Weapon>>,
    buddy: HashMap<String, Vec<Buddy>>,
}

pub async fn spawn(pool: PgPool) {
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        let handle = rt.spawn(async move {
            let mut interval = rt::time::interval(Duration::from_secs(60 * 10));

            let mut up_to_date = false;

            loop {
                interval.tick().await;

                let start = Instant::now();

                if let Err(e) = update(&mut up_to_date, pool.clone()).await {
                    error!(
                        "Dimbreath zzz update failed with {e} in {}s",
                        start.elapsed().as_secs_f64()
                    );
                } else {
                    info!(
                        "Dimbreath zzz update succeeded in {}s",
                        start.elapsed().as_secs_f64()
                    );
                }
            }
        });

        rt.block_on(handle).unwrap();
    });
}

async fn update(up_to_date: &mut bool, pool: PgPool) -> anyhow::Result<()> {
    if !Path::new("ZenlessData").exists() {
        Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/Dimbreath/ZenlessData",
            ])
            .output()
            .await?;

        *up_to_date = false;
    }

    let output = String::from_utf8(
        Command::new("git")
            .arg("pull")
            .current_dir("ZenlessData")
            .output()
            .await?
            .stdout,
    )?;

    if !output.contains("Already up to date.") {
        *up_to_date = false;
    }

    if *up_to_date {
        return Ok(());
    }

    let achievement_second_class_config: HashMap<String, Vec<AchieveSecondClass>> =
        serde_json::from_reader(BufReader::new(File::open(
            "ZenlessData/FileCfg/AchieveSecondClassConfigTemplateTb.json",
        )?))?;

    let achievement: HashMap<String, Vec<Achievement>> = serde_json::from_reader(BufReader::new(
        File::open("ZenlessData/FileCfg/AchievementTemplateTb.json")?,
    ))?;

    let once_reward: HashMap<String, Vec<Rewards>> = serde_json::from_reader(BufReader::new(
        File::open("ZenlessData/FileCfg/OnceRewardTemplateTb.json")?,
    ))?;

    let item: HashMap<String, Vec<Item>> = serde_json::from_reader(BufReader::new(File::open(
        "ZenlessData/FileCfg/ItemTemplateTb.json",
    )?))?;

    let avatar: HashMap<String, Vec<Avatar>> = serde_json::from_reader(BufReader::new(
        File::open("ZenlessData/FileCfg/AvatarBaseTemplateTb.json")?,
    ))?;

    let weapon: HashMap<String, Vec<Weapon>> = serde_json::from_reader(BufReader::new(
        File::open("ZenlessData/FileCfg/WeaponTemplateTb.json")?,
    ))?;

    let buddy: HashMap<String, Vec<Buddy>> = serde_json::from_reader(BufReader::new(File::open(
        "ZenlessData/FileCfg/BuddyBaseTemplateTb.json",
    )?))?;

    let configs = Configs {
        achievement_second_class_config,
        achievement,
        once_reward,
        item,
        avatar,
        weapon,
        buddy,
    };

    info!("Starting achievement series");
    achievement_series::update(&configs, &pool).await?;
    actix_web::rt::task::yield_now().await;

    info!("Starting achievements");
    achievements::update(&configs, &pool).await?;
    actix_web::rt::task::yield_now().await;

    info!("Starting avatars");
    avatars::update(&configs, &pool).await?;
    actix_web::rt::task::yield_now().await;

    info!("Starting w-engines");
    w_engines::update(&configs, &pool).await?;
    actix_web::rt::task::yield_now().await;

    info!("Starting bangboos");
    buddys::update(&configs, &pool).await?;
    actix_web::rt::task::yield_now().await;

    info!("Starting texts");
    texts::update(&configs, &pool).await?;
    actix_web::rt::task::yield_now().await;

    Ok(())
}
