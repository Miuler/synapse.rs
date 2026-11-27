//! Traduce los criterios de un tablero a WIQL. Los nombres de campo salen de aquí y cada valor va
//! entre comillas: nada de lo que se escriba o venga del proveedor puede cambiar la consulta.

use crate::domain::issue::filter::{CURRENT, IssueFilter, ME};
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind as Kind};

/// Estados que se dan por cerrados cuando el filtro no elige estados.
const CLOSED: [&str; 4] = ["Closed", "Cancelled", "Removed", "Done"];

/// Un literal de WIQL. La comilla simple se dobla; una línea nueva no tiene forma de escribirse.
fn literal(value: &str) -> Result<String, IssueError> {
    if value.chars().any(char::is_control) {
        return Err(IssueError::new(
            Kind::Rule,
            value.escape_default().to_string(),
        ));
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

fn list(values: &[&str]) -> Result<String, IssueError> {
    let literals = values
        .iter()
        .map(|v| literal(v))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(literals.join(", "))
}

/// Una condición por valor, unidas con `OR`.
fn any(field: &str, operator: &str, values: &[&str]) -> Result<Vec<String>, IssueError> {
    values
        .iter()
        .map(|v| Ok(format!("{field} {operator} {}", literal(v)?)))
        .collect()
}

fn group(conditions: Vec<String>) -> Option<String> {
    match conditions.len() {
        0 => None,
        1 => conditions.into_iter().next(),
        _ => Some(format!("({})", conditions.join(" OR "))),
    }
}

/// `current` son las rutas de las iteraciones en curso, para el valor `@current`.
pub fn compile(filter: &IssueFilter, current: &[String]) -> Result<String, IssueError> {
    // Sin espacios sobrantes ni valores vacíos
    let refs = |values: &[String]| -> Vec<String> {
        values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .collect()
    };
    let mut conditions = vec!["[System.TeamProject] = @project".to_string()];

    let states = refs(&filter.states);
    let states: Vec<&str> = states.iter().map(String::as_str).collect();
    conditions.push(if states.is_empty() {
        format!("[System.State] NOT IN ({})", list(&CLOSED)?)
    } else {
        format!("[System.State] IN ({})", list(&states)?)
    });

    let types = refs(&filter.types);
    let types: Vec<&str> = types.iter().map(String::as_str).collect();
    if !types.is_empty() {
        conditions.push(format!("[System.WorkItemType] IN ({})", list(&types)?));
    }

    let assignees = refs(&filter.assignees);
    let me = assignees.iter().any(|a| a.eq_ignore_ascii_case(ME));
    let others: Vec<&str> = assignees
        .iter()
        .map(String::as_str)
        .filter(|a| !a.eq_ignore_ascii_case(ME))
        .collect();
    let mut people = Vec::new();
    if me {
        people.push("[System.AssignedTo] = @me".to_string());
    }
    if !others.is_empty() {
        people.push(format!("[System.AssignedTo] IN ({})", list(&others)?));
    }
    conditions.extend(group(people));

    let mut iterations = Vec::new();
    for iteration in refs(&filter.iterations) {
        if iteration == CURRENT {
            if current.is_empty() {
                return Err(IssueError::new(Kind::NoCurrentIteration, ""));
            }
            iterations.extend(current.iter().cloned());
        } else {
            iterations.push(iteration);
        }
    }
    let iterations: Vec<&str> = iterations.iter().map(String::as_str).collect();
    conditions.extend(group(any("[System.IterationPath]", "UNDER", &iterations)?));

    let areas = refs(&filter.areas);
    let areas: Vec<&str> = areas.iter().map(String::as_str).collect();
    conditions.extend(group(any("[System.AreaPath]", "UNDER", &areas)?));

    let tags = refs(&filter.tags);
    let tags: Vec<&str> = tags.iter().map(String::as_str).collect();
    conditions.extend(group(any("[System.Tags]", "CONTAINS", &tags)?));

    Ok(format!(
        "SELECT [System.Id] FROM WorkItems\nWHERE {}\nORDER BY [System.ChangedDate] DESC",
        conditions.join("\n  AND ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn sin_criterios_trae_lo_abierto_del_proyecto() {
        assert_eq!(
            compile(&IssueFilter::default(), &[]).unwrap(),
            "SELECT [System.Id] FROM WorkItems\n\
             WHERE [System.TeamProject] = @project\n  \
             AND [System.State] NOT IN ('Closed', 'Cancelled', 'Removed', 'Done')\n\
             ORDER BY [System.ChangedDate] DESC"
        );
    }

    #[test]
    fn cada_criterio_admite_varios_valores_y_se_exigen_todos() {
        let filter = IssueFilter {
            types: some(&["Bug", "Task"]),
            states: some(&["Active"]),
            assignees: some(&["@Me", "ana@ejemplo.test"]),
            iterations: some(&[CURRENT, "P\\Sprint 9"]),
            areas: some(&["P\\Web"]),
            tags: some(&["urgente", " "]),
        };
        let query = compile(&filter, &some(&["P\\Sprint 1"])).unwrap();
        for condition in [
            "[System.State] IN ('Active')",
            "[System.WorkItemType] IN ('Bug', 'Task')",
            "([System.AssignedTo] = @me OR [System.AssignedTo] IN ('ana@ejemplo.test'))",
            "([System.IterationPath] UNDER 'P\\Sprint 1' OR [System.IterationPath] UNDER 'P\\Sprint 9')",
            "AND [System.AreaPath] UNDER 'P\\Web'",
            "AND [System.Tags] CONTAINS 'urgente'",
        ] {
            assert!(query.contains(condition), "{condition}\n{query}");
        }
        assert!(!query.contains("NOT IN"));
    }

    #[test]
    fn un_valor_no_puede_salirse_de_sus_comillas() {
        let filter = IssueFilter {
            tags: some(&["x' OR [System.Id] > 0 --"]),
            ..Default::default()
        };
        let query = compile(&filter, &[]).unwrap();
        assert!(query.contains("CONTAINS 'x'' OR [System.Id] > 0 --'"));
        let newline = IssueFilter {
            areas: some(&["a\nb"]),
            ..Default::default()
        };
        assert_eq!(compile(&newline, &[]).unwrap_err().kind, Kind::Rule);
    }

    #[test]
    fn la_iteracion_actual_exige_que_haya_una() {
        let filter = IssueFilter {
            iterations: some(&[CURRENT]),
            ..Default::default()
        };
        assert_eq!(
            compile(&filter, &[]).unwrap_err().kind,
            Kind::NoCurrentIteration
        );
    }
}
