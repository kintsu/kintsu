use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    SpannedToken,
    ast::ty::PathOrIdent,
    ctx::{
        Definition, FromNamedSource, NamespaceCtx, ResolvedType, WithSource,
        paths::NamedItemContext,
    },
    defs::{Span, Spanned},
    tokens::ToTokens,
};

#[derive(Clone)]
pub struct TypeRegistry {
    inner: Arc<
        Mutex<BTreeMap<super::paths::NamedItemContext, FromNamedSource<Spanned<ResolvedType>>>>,
    >,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::default(),
        }
    }

    fn with_lock<F, R>(
        &self,
        f: F,
    ) -> crate::Result<R>
    where
        F: FnOnce(
            &BTreeMap<super::paths::NamedItemContext, FromNamedSource<Spanned<ResolvedType>>>,
        ) -> R, {
        self.inner
            .lock()
            .map(|guard| f(&guard))
            .map_err(|_| {
                crate::Error::InternalError {
                    message: "TypeRegistry lock poisoned".into(),
                }
            })
    }

    fn with_lock_mut<F, R>(
        &self,
        f: F,
    ) -> crate::Result<R>
    where
        F: FnOnce(
            &mut BTreeMap<super::paths::NamedItemContext, FromNamedSource<Spanned<ResolvedType>>>,
        ) -> R, {
        self.inner
            .lock()
            .map(|mut guard| f(&mut guard))
            .map_err(|_| {
                crate::Error::InternalError {
                    message: "TypeRegistry lock poisoned".into(),
                }
            })
    }

    pub fn register(
        &self,
        context: &super::paths::RefContext,
        name: &SpannedToken![ident],
        kind: Definition,
        span: Span,
        source: PathBuf,
    ) -> crate::Result<()> {
        self.with_lock_mut(|inner| {
            let span = span.span();

            let path = context.item(name.clone());

            if inner.contains_key(&path) {
                return Err(crate::Error::DuplicateType {
                    name: path.display(),
                }
                .with_span(span.start, span.end));
            }
            inner.insert(
                path.clone(),
                ResolvedType {
                    kind,
                    qualified_path: path,
                }
                .with_source_and_span(source, Span::Known(span.clone())),
            );
            Ok(())
        })?
    }

    #[tracing::instrument(
        level = "TRACE",
        target = "type-registry",
        skip(self, context, reference, ns),
        fields(
            context = context.display(),
            reference = %reference.display(),
            ns = ns.ctx.display(),
        )
    )]
    pub fn resolve(
        &self,
        context: &super::paths::RefContext,
        reference: &PathOrIdent,
        ns: &NamespaceCtx,
    ) -> Option<FromNamedSource<Spanned<ResolvedType>>> {
        self.with_lock(|inner| {
            let uses_candidates =
                super::graph::extract::TypeExtractor::generate_candidates(reference, context, ns);
            for candidate in &uses_candidates {
                tracing::trace!(
                    candidate = candidate.display(),
                    candidate_path = candidate.path().display(),
                    "attempting candidate"
                );
                if let Some(found) = inner.get(candidate) {
                    return Some(found.clone());
                }
            }
            None
        })
        .ok()
        .flatten()
    }

    pub fn resolve_if_valid(
        &self,
        context: &super::paths::RefContext,
        reference: &PathOrIdent,
        ns: &NamespaceCtx,
    ) -> Option<FromNamedSource<Spanned<ResolvedType>>> {
        self.resolve(context, reference, ns)
    }

    pub fn resolve_required(
        &self,
        context: &super::paths::RefContext,
        reference: &PathOrIdent,
        ns: &NamespaceCtx,
    ) -> crate::Result<FromNamedSource<Spanned<ResolvedType>>> {
        self.resolve(context, reference, ns)
            .ok_or_else(|| {
                let type_name = match reference {
                    PathOrIdent::Ident(name) => name.borrow_string().clone(),
                    PathOrIdent::Path(path) => path.display(),
                };
                crate::Error::UndefinedType { name: type_name }
            })
    }

    /// Direct lookup by NamedItemContext
    pub fn get(
        &self,
        context: &NamedItemContext,
    ) -> Option<FromNamedSource<Spanned<ResolvedType>>> {
        self.with_lock(|inner| inner.get(context).cloned())
            .ok()
            .flatten()
    }

    pub fn is_valid(
        &self,
        context: &super::paths::RefContext,
        reference: &PathOrIdent,
        ns: &NamespaceCtx,
    ) -> bool {
        self.resolve(context, reference, ns)
            .is_some()
    }

    pub fn all_types(&self) -> Vec<(NamedItemContext, PathBuf, Span)> {
        self.with_lock(|inner| {
            inner
                .iter()
                .map(|(path, spanned)| {
                    (
                        path.clone(),
                        spanned.source().clone(),
                        spanned.span().clone(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::items::TypeDef,
        ctx::{RefContext, RefOrItemContext, WithSource},
        tokens::{IdentToken, SemiToken},
    };

    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = TypeRegistry::new();
        assert_eq!(
            registry.all_types().len(),
            0,
            "New registry should be empty"
        );
    }

    #[test]
    fn test_registry_thread_safety() {
        use std::thread;

        let registry = TypeRegistry::new();
        let mut handles = vec![];

        for _ in 0..5 {
            let registry_clone = registry.clone();
            let handle = thread::spawn(move || {
                let _ = registry_clone.all_types();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn test_registry_resolve() {
        let registry = TypeRegistry::new();
        let ctx = RefContext::new("test_pkg".into(), vec!["types".into()]);
        registry
            .register(
                // test_pkg::types::Foo
                &ctx,
                &Spanned::call_site(IdentToken::new("Foo".into())),
                Definition::TypeAlias(Arc::new(TypeDef {
                    meta: vec![],
                    def: crate::tst::basic_smoke("type Foo = i32").unwrap(),
                    end: Spanned::call_site(SemiToken::new()),
                })),
                Span::CallSite,
                "foo.ks".into(),
            )
            .unwrap();

        for (path, i) in [
            (
                "test_pkg::types::Foo",
                RefOrItemContext::Ref(RefContext {
                    package: "test_pkg".into(),
                    namespace: vec![],
                })
                .with_source("foo.ks".into()),
            ),
            (
                "types::Foo",
                RefOrItemContext::Ref(ctx.clone()).with_source("foo.ks".into()),
            ),
            (
                "Foo",
                RefOrItemContext::Item(ctx.item(Spanned::call_site(IdentToken::new("Foo".into()))))
                    .with_source("foo.ks".into()),
            ),
        ] {
            let mut ns = crate::tst::create_raw_namespace("user");

            ns.imports.push(i);

            let _ = registry
                .resolve(&ctx, &crate::tst::basic_smoke(path).unwrap(), &ns)
                .expect("Failed to resolve type");
        }
    }
}
