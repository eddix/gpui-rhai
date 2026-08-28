use crate::{CompiledUi, RuntimeEngine, RuntimeError, ScriptGeneration, UiContext, UiNode};

#[derive(Clone)]
pub struct LiveScript {
    active: CompiledUi,
    root: UiNode,
}

impl LiveScript {
    /// Compile, render, and activate the initial script.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError`] when compilation or initial rendering fails.
    pub fn load(runtime: &mut RuntimeEngine, source: &str) -> Result<Self, RuntimeError> {
        let compiled = runtime.compile(source)?;
        let root = runtime.render(&compiled)?;
        Ok(Self {
            active: compiled,
            root,
        })
    }

    /// Compile, render, and activate an initial script using `view(ctx)`.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError`] when compilation or initial rendering fails.
    pub fn load_with_context(
        runtime: &mut RuntimeEngine,
        source: &str,
        context: UiContext,
    ) -> Result<Self, RuntimeError> {
        let compiled = runtime.compile(source)?;
        let root = runtime.render_with_context(&compiled, context)?;
        Ok(Self {
            active: compiled,
            root,
        })
    }

    #[must_use]
    pub fn root(&self) -> &UiNode {
        &self.root
    }

    #[must_use]
    pub fn generation(&self) -> ScriptGeneration {
        self.active.generation()
    }

    pub fn reload(&mut self, runtime: &mut RuntimeEngine, source: &str) -> ReloadOutcome {
        let candidate = match runtime.compile(source) {
            Ok(candidate) => candidate,
            Err(error) => return ReloadOutcome::Rejected { error },
        };
        match runtime.render(&candidate) {
            Ok(root) => {
                self.active = candidate;
                self.root = root;
                ReloadOutcome::Applied {
                    generation: self.active.generation(),
                }
            }
            Err(error) => ReloadOutcome::Rejected { error },
        }
    }

    pub fn reload_with_context(
        &mut self,
        runtime: &mut RuntimeEngine,
        source: &str,
        context: UiContext,
    ) -> ReloadOutcome {
        let candidate = match runtime.compile(source) {
            Ok(candidate) => candidate,
            Err(error) => return ReloadOutcome::Rejected { error },
        };
        match runtime.render_with_context(&candidate, context) {
            Ok(root) => {
                self.active = candidate;
                self.root = root;
                ReloadOutcome::Applied {
                    generation: self.active.generation(),
                }
            }
            Err(error) => ReloadOutcome::Rejected { error },
        }
    }
}

#[derive(Debug)]
pub enum ReloadOutcome {
    Applied { generation: ScriptGeneration },
    Rejected { error: RuntimeError },
}

impl ReloadOutcome {
    #[must_use]
    pub const fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_reload_preserves_last_good_tree_and_generation() {
        let mut runtime = RuntimeEngine::new();
        let mut live = LiveScript::load(&mut runtime, "fn view() { text(\"good\") }").unwrap();
        let generation = live.generation();
        let root = live.root().clone();

        assert!(matches!(
            live.reload(&mut runtime, "fn view() { text("),
            ReloadOutcome::Rejected { .. }
        ));
        assert_eq!(live.generation(), generation);
        assert_eq!(live.root(), &root);

        assert!(matches!(
            live.reload(&mut runtime, "fn view() { throw \"bad render\"; }"),
            ReloadOutcome::Rejected { .. }
        ));
        assert_eq!(live.generation(), generation);
        assert_eq!(live.root(), &root);
        assert!(runtime.is_current(generation));
    }

    #[test]
    fn successful_reload_atomically_replaces_tree() {
        let mut runtime = RuntimeEngine::new();
        let mut live = LiveScript::load(&mut runtime, "fn view() { text(\"one\") }").unwrap();
        let first = live.generation();
        assert!(
            live.reload(&mut runtime, "fn view() { text(\"two\") }")
                .is_applied()
        );
        assert!(live.generation() > first);
        assert!(matches!(
            live.root().kind(),
            crate::UiNodeKind::Text { text } if text == "two"
        ));
        assert!(live.root().source().is_some());
    }
}
