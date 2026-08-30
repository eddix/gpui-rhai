use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeBudgets {
    pub retained_nodes: usize,
    pub event_handlers: usize,
    pub formal_components: usize,
    pub effects: usize,
    pub signals: usize,
    pub element_refs: usize,
    pub canvas_commands: usize,
}

impl Default for RuntimeBudgets {
    fn default() -> Self {
        Self {
            retained_nodes: 100_000,
            event_handlers: 100_000,
            formal_components: 10_000,
            effects: 4_096,
            signals: 16_384,
            element_refs: 16_384,
            canvas_commands: 100_000,
        }
    }
}

impl RuntimeBudgets {
    /// Validate that every resource class permits at least one entry.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeBudgetError::ZeroLimit`] for a disabled core class.
    pub fn validate(&self) -> Result<(), RuntimeBudgetError> {
        for (resource, limit) in [
            ("retained_nodes", self.retained_nodes),
            ("event_handlers", self.event_handlers),
            ("formal_components", self.formal_components),
            ("effects", self.effects),
            ("signals", self.signals),
            ("element_refs", self.element_refs),
            ("canvas_commands", self.canvas_commands),
        ] {
            if limit == 0 {
                return Err(RuntimeBudgetError::ZeroLimit(resource));
            }
        }
        Ok(())
    }

    pub(crate) fn check(
        resource: &'static str,
        actual: usize,
        limit: usize,
    ) -> Result<(), RuntimeBudgetError> {
        (actual <= limit)
            .then_some(())
            .ok_or(RuntimeBudgetError::Exceeded {
                resource,
                actual,
                limit,
            })
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RuntimeBudgetError {
    #[error("runtime budget `{0}` must be greater than zero")]
    ZeroLimit(&'static str),
    #[error("runtime budget `{resource}` exceeded: {actual} > {limit}")]
    Exceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budgets_are_valid_and_limits_are_inclusive() {
        let budgets = RuntimeBudgets::default();
        budgets.validate().unwrap();
        RuntimeBudgets::check("signals", budgets.signals, budgets.signals).unwrap();
        assert!(matches!(
            RuntimeBudgets::check("signals", budgets.signals + 1, budgets.signals),
            Err(RuntimeBudgetError::Exceeded { .. })
        ));
    }
}
