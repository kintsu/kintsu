use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    Definitions, Ident,
    declare::{DeclNamespace, DeclarationBundle, TypeRegistryDeclaration},
    namespace::Namespace,
};

#[derive(Default)]
pub struct Context {
    pub namespaces: BTreeMap<Ident, Namespace>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish(&mut self) -> crate::Result<()> {
        for ns in self.namespaces.values_mut() {
            ns.check()?;
        }
        Ok(())
    }

    pub fn load_from_source(
        &mut self,
        source: PathBuf,
    ) -> crate::Result<()> {
        self.with_definition(Definitions::load_from_path(source)?)
    }

    pub fn load_from_sources(
        &mut self,
        sources: Vec<PathBuf>,
    ) -> crate::Result<()> {
        for source in sources {
            self.load_from_source(source)?;
        }
        Ok(())
    }

    pub fn get_or_create_ns(
        &mut self,
        ns: &Ident,
    ) -> &mut Namespace {
        if !self.namespaces.contains_key(ns) {
            self.namespaces
                .insert(ns.clone(), Namespace::new(ns.clone()));
        }

        self.namespaces.get_mut(ns).unwrap()
    }

    pub fn with_definition(
        &mut self,
        def: Definitions,
    ) -> crate::Result<()> {
        match def {
            Definitions::NamespaceV1(ns) => {
                match self.namespaces.get_mut(&ns.name) {
                    Some(value) => {
                        value.merge(ns)?;
                    },
                    None => {
                        self.namespaces.insert(ns.name.clone(), ns);
                    },
                };
                Ok(())
            },
            def => {
                self.get_or_create_ns(def.namespace())
                    .with_definition(def)
            },
        }
    }

    pub fn with_definitions(
        &mut self,
        defs: Vec<Definitions>,
    ) -> crate::Result<()> {
        for def in defs {
            self.with_definition(def)?;
        }
        Ok(())
    }

    /// Convert this context to a DeclarationBundle using the new parser types.
    pub fn to_declaration_bundle(
        &self,
        package: impl Into<String>,
    ) -> DeclarationBundle {
        let package = package.into();
        let mut namespaces = BTreeMap::new();

        for (ident, ns) in &self.namespaces {
            let decl_ns: DeclNamespace = ns.into();
            namespaces.insert(ident.to_string(), decl_ns);
        }

        DeclarationBundle {
            root: TypeRegistryDeclaration {
                package,
                namespaces,
                external_refs: Default::default(),
            },
            dependencies: Default::default(),
        }
    }
}

/// A context that works directly with parser declaration types.
pub struct DeclContext {
    pub bundle: DeclarationBundle,
}

impl DeclContext {
    pub fn new(package: impl Into<String>) -> Self {
        Self {
            bundle: DeclarationBundle {
                root: TypeRegistryDeclaration::new(package.into()),
                dependencies: Default::default(),
            },
        }
    }

    pub fn from_bundle(bundle: DeclarationBundle) -> Self {
        Self { bundle }
    }

    pub fn from_legacy_context(
        ctx: &Context,
        package: impl Into<String>,
    ) -> Self {
        Self {
            bundle: ctx.to_declaration_bundle(package),
        }
    }

    pub fn root(&self) -> &TypeRegistryDeclaration {
        &self.bundle.root
    }

    pub fn root_mut(&mut self) -> &mut TypeRegistryDeclaration {
        &mut self.bundle.root
    }
}
