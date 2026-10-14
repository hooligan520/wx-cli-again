//! `wx cache` —— 把加密分片解密成明文副本，统一落到 `cache_dir()`。
//!
//! ## 为什么需要这条命令
//!
//! daemon 的明文缓存是**实现细节，不是契约**：它首选 `CacheMode::Online`
//! （对加密源库做 SQLCipher 在线打开，**不产生明文副本**），只有它自己认为
//! 需要的少部分库才落盘。所以「拿别的语言/工具直接对**全部**消息分片跑 SQL」
//! 这件事，靠现有命令做不到 —— 只能自己再写一个解密器，而那就是第二份页布局
//! 实现、第二个事实来源。这条命令把这件事收回到 `wx` 里。
//!
//! ## 两条不能破的规矩
//!
//! 1. **只补齐，绝不覆盖。** 已存在的明文可能是 daemon 落的，它额外合并了
//!    `-wal` 增量，比只解主库文件更全；覆盖它 = 静默丢最新消息。
//!    要刷新某个分片，先自己删掉那个文件再跑。
//! 2. **不写自己的索引文件。** 文件名就是 `<md5(rel)>.db`，存在性即状态：
//!    幂等、可重复跑、崩溃也不会留下半截状态机。少一个文件就少一个「源」。
//!
//! 不依赖 daemon：直接读 `config.json` + `all_keys.json`，daemon 没起也能跑。

use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::cli::CacheAction;
use crate::config;
use crate::crypto;
use crate::daemon::cache::cache_file_name;
use crate::daemon::{extract_keys, is_msg_db_key};

/// `pull` 默认补齐：会话 + 通讯录 + 所有聊天分片。
///
/// 刻意**不含** `message_fts.db`（约 1 GB，仅供全文搜索）、`media_0.db`（语音）、
/// `biz_message_*`（公众号）—— 对聊天分析没用，白占磁盘。要就显式 `--all`。
const DEFAULT_EXTRA: &[&str] = &["session/session.db", "contact/contact.db"];

fn is_default_target(rel: &str) -> bool {
    if DEFAULT_EXTRA.contains(&rel) {
        return true;
    }
    // 必须是真的消息分片 `message/message_<digits>.db`；
    // `is_msg_db_key` 只认前缀/后缀，不校验数字段，这里补上。
    is_msg_db_key(rel)
        && rel
            .strip_prefix("message/message_")
            .and_then(|s| s.strip_suffix(".db"))
            .map_or(false, |n| {
                !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
            })
}

fn cache_file_path(rel: &str) -> PathBuf {
    config::cache_dir().join(cache_file_name(rel))
}

fn mb(path: &Path) -> f64 {
    std::fs::metadata(path).map(|m| m.len() as f64 / 1e6).unwrap_or(0.0)
}

/// 读取 `db_dir` 与 `{rel: enc_key}`。
fn load_state() -> Result<(PathBuf, BTreeMap<String, String>)> {
    let cfg = config::load_config()?;
    let content = std::fs::read_to_string(&cfg.keys_file)
        .with_context(|| format!("读取密钥文件 {} 失败", cfg.keys_file.display()))?;
    let raw: serde_json::Value =
        serde_json::from_str(&content).context("all_keys.json 格式错误")?;
    let keys: BTreeMap<String, String> = extract_keys(&raw).into_iter().collect();
    Ok((cfg.db_dir, keys))
}

pub fn cmd_cache(action: CacheAction) -> Result<()> {
    match action {
        CacheAction::List { json } => cmd_list(json),
        CacheAction::Pull {
            all,
            shards,
            refresh,
            json,
        } => cmd_pull(all, shards, refresh, json),
    }
}

// --------------------------------------------------------------------- list

fn cmd_list(json_out: bool) -> Result<()> {
    let (db_dir, keys) = load_state()?;
    let mut rows = Vec::new();
    let mut plain_total = 0.0f64;

    for rel in keys.keys() {
        let src = db_dir.join(rel);
        if !src.is_file() {
            continue;
        }
        let plain = cache_file_path(rel);
        let present = plain.is_file();
        if present {
            plain_total += mb(&plain);
        }
        rows.push(json!({
            "rel": rel,
            "src_mb": round1(mb(&src)),
            "has_key": true,
            "plain": present,
            "plain_mb": if present { round1(mb(&plain)) } else { 0.0 },
        }));
    }

    let payload = json!({
        "db_dir": db_dir,
        "cache_dir": config::cache_dir(),
        "shards": rows.len(),
        "plain": rows.iter().filter(|r| r["plain"] == json!(true)).count(),
        "plain_mb": round1(plain_total),
        "rows": rows,
    });

    if json_out {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("db_dir    : {}", db_dir.display());
    println!("cache_dir : {}", config::cache_dir().display());
    println!(
        "\n{:<32} {:>9} {:>5} {:>9}",
        "REL", "源(MB)", "明文", "明文(MB)"
    );
    for r in payload["rows"].as_array().unwrap() {
        let present = r["plain"] == json!(true);
        println!(
            "{:<32} {:>9.1} {:>5} {:>9}",
            r["rel"].as_str().unwrap_or(""),
            r["src_mb"].as_f64().unwrap_or(0.0),
            if present { "有" } else { "-" },
            if present {
                format!("{:.1}", r["plain_mb"].as_f64().unwrap_or(0.0))
            } else {
                "-".to_string()
            },
        );
    }
    println!(
        "\n共 {} 个有密钥的分片，其中 {} 个已有明文（{:.1} MB）",
        payload["shards"], payload["plain"], payload["plain_mb"]
    );
    Ok(())
}

// --------------------------------------------------------------------- pull

fn cmd_pull(all: bool, shards: Vec<String>, refresh: bool, json_out: bool) -> Result<()> {
    // `--refresh` 会覆盖已有明文（丢掉 daemon 的 -wal 合并），所以必须点名分片：
    // 不允许配 `--all` 一把梭，否则一次命令就能把热库的明文降级成无 WAL 版本。
    if refresh && shards.is_empty() {
        anyhow::bail!(
            "--refresh 必须配合 --shard <REL> 使用。\n\
             daemon 正在管理的热库（message_0/1/2、session、contact）请交给 daemon 自己刷新；\n\
             只有确认某个冷分片已经过期、需要强制重解时才点名它。"
        );
    }

    let (db_dir, keys) = load_state()?;
    let cache_dir = config::cache_dir();
    std::fs::create_dir_all(&cache_dir)
        .with_context(|| format!("创建明文目录失败: {}", cache_dir.display()))?;

    let wanted: Vec<String> = if !shards.is_empty() {
        // 归一化，容忍用户把 `\` 写成 `/`
        shards.iter().map(|s| s.replace('\\', "/")).collect()
    } else if all {
        keys.keys().cloned().collect()
    } else {
        keys.keys()
            .filter(|r| is_default_target(r))
            .cloned()
            .collect()
    };

    let mut done: Vec<serde_json::Value> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut no_src: Vec<String> = Vec::new();
    let mut no_key: Vec<String> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();

    for rel in wanted {
        let plain = cache_file_path(&rel);

        // 规矩 1：已存在就跳过 —— 不覆盖 daemon 那份（带 -wal 合并）。
        // 只有显式 `--refresh --shard X` 才允许点名覆盖。
        if plain.is_file() && !refresh {
            skipped.push(rel);
            continue;
        }
        let src = db_dir.join(&rel);
        if !src.is_file() {
            no_src.push(rel);
            continue;
        }
        let Some(key) = keys
            .get(&rel)
            .and_then(|h| crate::scanner::decode_key_hex_pub(h))
        else {
            no_key.push(rel);
            continue;
        };

        // 先验密钥再解：密钥不对时解出来的是垃圾，宁可不落盘
        if !crypto::validate_raw_key_for_db(&src, &key) {
            failed.push((rel, "密钥不匹配（页 1 HMAC 校验失败）".into()));
            continue;
        }

        let tmp = cache_dir.join(format!("{}.part", cache_file_name(&rel)));
        let result = crypto::full_decrypt(&src, &tmp, &key).and_then(|_| {
            std::fs::rename(&tmp, &plain)
                .with_context(|| format!("原子替换失败: {}", plain.display()))
        });
        match result {
            Ok(()) => done.push(json!({
                "rel": rel,
                "plain": plain.to_string_lossy(),
                "plain_mb": round1(mb(&plain)),
            })),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                failed.push((rel, e.to_string()));
            }
        }
    }

    let payload = json!({
        "cache_dir": cache_dir,
        "written": done.len(),
        "written_mb": round1(done.iter().map(|d| d["plain_mb"].as_f64().unwrap_or(0.0)).sum()),
        "skipped_exists": skipped,
        "no_source": no_src,
        "no_key": no_key,
        "failed": failed.iter().map(|(r, e)| json!({"rel": r, "error": e})).collect::<Vec<_>>(),
        "written_shards": done,
    });

    if json_out {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    for d in &done {
        println!("  + {:<32} {:>9.1} MB", d["rel"].as_str().unwrap_or(""), d["plain_mb"]);
    }
    println!(
        "\n明文目录: {}\n新解 {} 个（{:.1} MB）/ 已存在跳过 {} / 无源文件 {} / 无密钥 {} / 失败 {}",
        cache_dir.display(),
        payload["written"],
        payload["written_mb"],
        skipped.len(),
        no_src.len(),
        no_key.len(),
        failed.len(),
    );
    for (rel, e) in &failed {
        eprintln!("  ! {} : {}", rel, e);
    }
    if payload["written"] == json!(0) && failed.is_empty() {
        println!("（没有需要补齐的分片）");
    }
    Ok(())
}

fn round1(v: f64) -> f64 {
    let r = (v * 10.0).round() / 10.0;
    // 归一化 -0.0：空集合求和会打印成 "-0.0"，看着像 bug
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::is_default_target;

    #[test]
    fn default_target_covers_chat_shards_and_core_dbs() {
        for rel in [
            "message/message_0.db",
            "message/message_11.db",
            "session/session.db",
            "contact/contact.db",
        ] {
            assert!(is_default_target(rel), "{rel} 应在默认补齐范围内");
        }
    }

    #[test]
    fn default_target_excludes_heavy_side_dbs() {
        for rel in [
            "message/message_fts.db",
            "message/message_resource.db",
            "message/biz_message_0.db",
            "media/media_0.db",
            "sns/sns.db",
        ] {
            assert!(!is_default_target(rel), "{rel} 不该被默认补齐（占空间且聊天分析用不到）");
        }
    }

    #[test]
    fn default_target_rejects_non_numeric_shard() {
        assert!(!is_default_target("message/message_x.db"));
        assert!(!is_default_target("message/message_.db"));
    }
}
