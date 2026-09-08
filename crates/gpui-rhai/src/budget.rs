use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeBudgets {
    pub retained_nodes: usize,
    pub event_handlers: usize,
    pub formal_components: usize,
    pub effects: usize,
    pub signals: usize,
    pub element_refs: usize,
    pub timers: usize,
    pub layers: usize,
    pub canvas_scenes: usize,
    pub canvas_commands: usize,
    pub virtual_data_items: usize,
    pub virtual_realized_items: usize,
    pub background_tasks: usize,
    pub subscriptions: usize,
    pub image_decodes: usize,
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
            timers: 16_384,
            layers: 1_024,
            canvas_scenes: 4_096,
            canvas_commands: 100_000,
            virtual_data_items: 100_000,
            virtual_realized_items: 10_000,
            background_tasks: 256,
            subscriptions: 256,
            image_decodes: 256,
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
            ("timers", self.timers),
            ("layers", self.layers),
            ("canvas_scenes", self.canvas_scenes),
            ("canvas_commands", self.canvas_commands),
            ("virtual_data_items", self.virtual_data_items),
            ("virtual_realized_items", self.virtual_realized_items),
            ("background_tasks", self.background_tasks),
            ("subscriptions", self.subscriptions),
            ("image_decodes", self.image_decodes),
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
