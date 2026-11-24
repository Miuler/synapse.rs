use serde::Serialize;

/// Cuota de la cuenta con la que trabaja un agente (no el contexto de una conversación).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentQuota {
    pub groups: Vec<QuotaGroup>,
    /// Cuándo se obtuvo (milisegundos desde la época Unix).
    pub fetched_at: i64,
}

/// Modelos que comparten límites. Un agente con un solo grupo no necesita nombre.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuotaGroup {
    pub name: String,
    pub models: Vec<String>,
    pub windows: Vec<QuotaWindow>,
}

/// Límite de una ventana de tiempo.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuotaWindow {
    /// `5h`, `weekly` o el identificador que dé el agente.
    pub window: String,
    /// Parte que queda, de 0 a 1.
    pub remaining: f64,
    /// Milisegundos desde la época Unix; `None` si el agente no lo indica.
    pub resets_at: Option<i64>,
}

impl AgentQuota {
    /// El dato ya no vale: es más viejo que `max_age_ms` o alguna ventana se reinició después.
    pub fn is_stale(&self, now: i64, max_age_ms: i64) -> bool {
        now - self.fetched_at >= max_age_ms
            || self
                .groups
                .iter()
                .flat_map(|g| &g.windows)
                .any(|w| w.resets_at.is_some_and(|r| r > self.fetched_at && r <= now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caduca_por_edad_o_al_reiniciarse_una_ventana() {
        let quota = |resets_at| AgentQuota {
            groups: vec![QuotaGroup {
                name: "g".into(),
                models: vec![],
                windows: vec![QuotaWindow { window: "5h".into(), remaining: 0.5, resets_at }],
            }],
            fetched_at: 1_000,
        };
        assert!(!quota(None).is_stale(1_500, 1_000));
        assert!(quota(None).is_stale(2_000, 1_000));
        assert!(quota(Some(1_200)).is_stale(1_500, 1_000));
        assert!(!quota(Some(9_000)).is_stale(1_500, 1_000));
        // Un reinicio anterior al dato no lo caduca una y otra vez
        assert!(!quota(Some(500)).is_stale(1_500, 1_000));
    }
}
