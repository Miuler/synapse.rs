//! Cuota de la cuenta de los agentes que no la informan por ACP. Antigravity: `agy --print /usage`, que
//! responde con datos estructurados sin gastar cuota ni tocar la conversación.

use super::local_cli;
use crate::application::integration::service::AgentQuotaSource;
use crate::domain::agent::{AgentQuota, QuotaGroup, QuotaWindow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const ANTIGRAVITY: &str = "antigravity";
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Default)]
pub struct AgentQuotaSourceImpl {
    /// El bloqueo se mantiene durante la consulta: quien llega a la vez espera esa misma respuesta.
    cache: Arc<Mutex<HashMap<String, AgentQuota>>>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

impl AgentQuotaSource for AgentQuotaSourceImpl {
    async fn quota(&self, agent_id: &str, max_age_ms: i64) -> Result<Option<AgentQuota>, String> {
        if agent_id != ANTIGRAVITY {
            return Ok(None);
        }
        let asked_at = now_ms();
        let mut cache = self.cache.lock().await;
        if let Some(q) = cache.get(agent_id)
            && (q.fetched_at >= asked_at || !q.is_stale(now_ms(), max_age_ms))
        {
            return Ok(Some(q.clone()));
        }
        let quota = fetch_antigravity().await?;
        cache.insert(agent_id.to_string(), quota.clone());
        Ok(Some(quota))
    }
}

async fn fetch_antigravity() -> Result<AgentQuota, String> {
    let launch = local_cli::find(ANTIGRAVITY)
        .ok_or("Antigravity no está en la lista de agentes")?
        .launch()?;
    let mut cmd = tokio::process::Command::new(launch.command);
    // Sin shell y con argumentos fijos: la interfaz solo elige el agente
    cmd.args(["--print", "/usage", "--output-format", "json"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        // Fuera de la bóveda: no es parte de la conversación
        .current_dir(std::env::temp_dir())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let output = tokio::time::timeout(TIMEOUT, cmd.output())
        .await
        .map_err(|_| "agy no respondió a tiempo".to_string())?
        .map_err(|e| format!("No se pudo ejecutar agy: {e}"))?;
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| "agy no devolvió su cuota en JSON".to_string())?;
    parse_antigravity(&json, now_ms()).ok_or_else(|| "agy no informó su cuota".into())
}

/// `command.data` de `agy --print /usage --output-format json`. Lo que falte se omite.
fn parse_antigravity(json: &Value, fetched_at: i64) -> Option<AgentQuota> {
    let groups: Vec<QuotaGroup> = json
        .pointer("/command/data/groups")?
        .as_array()?
        .iter()
        .filter_map(|g| {
            let windows: Vec<QuotaWindow> = g
                .get("buckets")?
                .as_array()?
                .iter()
                .filter_map(|b| {
                    Some(QuotaWindow {
                        window: b.get("window")?.as_str()?.to_string(),
                        remaining: b.get("remaining_fraction")?.as_f64()?.clamp(0.0, 1.0),
                        // `1970-01-01…` es «sin fecha»
                        resets_at: b
                            .get("reset_time")
                            .and_then(Value::as_str)
                            .and_then(rfc3339_ms)
                            .filter(|ms| *ms > 0),
                    })
                })
                .collect();
            (!windows.is_empty()).then(|| QuotaGroup {
                name: text(g, "name"),
                // «Models within this group: A, B»
                models: text(g, "description")
                    .split_once(':')
                    .map(|(_, list)| {
                        list.split(',').map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).collect()
                    })
                    .unwrap_or_default(),
                windows,
            })
        })
        .collect();
    (!groups.is_empty()).then_some(AgentQuota { groups, fetched_at })
}

/// Límites que Claude adjunta a `usage_update` (`_meta["_claude/rateLimit"]`): ahí viene lo **usado**.
pub(super) fn claude_quota(update: &Value) -> Option<AgentQuota> {
    let info = update.get("_meta")?.get("_claude/rateLimit")?;
    let window = |id: &str, v: &Value| {
        Some(QuotaWindow {
            window: match id {
                "five_hour" => "5h".to_string(),
                "seven_day" => "weekly".to_string(),
                other => other.to_string(),
            },
            remaining: (1.0 - v.get("utilization")?.as_f64()?).clamp(0.0, 1.0),
            resets_at: v.get("resetsAt").and_then(Value::as_i64).map(|s| s * 1000),
        })
    };
    let windows: Vec<QuotaWindow> = match info.get("unifiedWindows").and_then(Value::as_object) {
        Some(all) => all.iter().filter_map(|(id, v)| window(id, v)).collect(),
        None => window(info.get("rateLimitType")?.as_str()?, info).into_iter().collect(),
    };
    (!windows.is_empty()).then(|| AgentQuota {
        groups: vec![QuotaGroup { name: String::new(), models: vec![], windows }],
        fetched_at: now_ms(),
    })
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// Milisegundos de `AAAA-MM-DDTHH:MM:SS[.fff]Z`, el único formato que usa agy.
fn rfc3339_ms(s: &str) -> Option<i64> {
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let nums = |t: &str, sep: char| -> Option<Vec<i64>> {
        t.split(sep).map(|p| p.parse().ok()).collect()
    };
    let d = nums(date, '-')?;
    let t = nums(time.split('.').next()?, ':')?;
    let ([y, m, day], [h, min, sec]) = (<[i64; 3]>::try_from(d).ok()?, <[i64; 3]>::try_from(t).ok()?);
    // Días desde 1970 del calendario gregoriano (el año empieza en marzo)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h * 3_600 + min * 60 + sec) * 1_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lee_la_cuota_de_agy() {
        let quota = parse_antigravity(
            &json!({"command": {"name": "usage", "data": {"groups": [
                {"name": "Gemini Models", "description": "Models within this group: Gemini 4 Neon, Gemini 4 Hydrogen", "buckets": [
                    {"id": "gemini-weekly", "window": "weekly", "remaining_fraction": 0.58, "reset_time": "2026-11-30T20:03:09Z"},
                    {"id": "roto", "window": "5h"}]},
                {"name": "Claude and GPT models", "buckets": [
                    {"window": "5h", "remaining_fraction": 1, "reset_time": "1970-01-01T00:00:00Z"}]},
                {"name": "Vacío", "buckets": []}]}}}),
            7,
        )
        .unwrap();
        assert_eq!(quota.fetched_at, 7);
        assert_eq!(quota.groups.len(), 2);
        assert_eq!(quota.groups[0].models, ["Gemini 4 Neon", "Gemini 4 Hydrogen"]);
        assert_eq!(quota.groups[0].windows.len(), 1);
        assert_eq!(quota.groups[0].windows[0].resets_at, Some(1_796_068_989_000));
        assert_eq!(quota.groups[1].windows[0].resets_at, None);
        assert_eq!(parse_antigravity(&json!({"response": "texto"}), 0), None);
    }

    #[test]
    fn lee_los_limites_de_claude_como_lo_que_queda() {
        let quota = claude_quota(&json!({"_meta": {"_claude/rateLimit": {
            "status": "allowed", "unifiedWindows": {
                "five_hour": {"utilization": 0.24, "resetsAt": 1_795_573_200},
                "seven_day": {"utilization": 0.3, "resetsAt": 1_796_086_800}}}}}))
        .unwrap();
        let w = &quota.groups[0].windows;
        assert_eq!((w[0].window.as_str(), w[1].window.as_str()), ("5h", "weekly"));
        assert!((w[0].remaining - 0.76).abs() < 1e-9);
        assert_eq!(w[0].resets_at, Some(1_795_573_200_000));
        // Sin porcentaje no hay cuota que enseñar
        assert_eq!(claude_quota(&json!({"_meta": {"_claude/rateLimit": {"status": "allowed", "rateLimitType": "five_hour"}}})), None);
        assert_eq!(claude_quota(&json!({"used": 1})), None);
    }
}
