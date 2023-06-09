use cairo_lang_defs::ids::{ExternTypeId, GenericKind, LanguageElementId};
use cairo_lang_diagnostics::{Diagnostics, Maybe, ToMaybe};
use cairo_lang_proc_macros::DebugWithDb;
use cairo_lang_syntax::node::TypedSyntaxNode;

use super::generics::semantic_generic_params;
use crate::db::SemanticGroup;
use crate::diagnostic::SemanticDiagnosticKind::*;
use crate::diagnostic::SemanticDiagnostics;
use crate::resolve::Resolver;
use crate::substitution::SemanticRewriter;
use crate::{GenericParam, SemanticDiagnostic};

#[cfg(test)]
#[path = "extern_type_test.rs"]
mod test;

// Declaration.
#[derive(Clone, Debug, PartialEq, Eq, DebugWithDb)]
#[debug_db(dyn SemanticGroup + 'static)]
pub struct ExternTypeDeclarationData {
    diagnostics: Diagnostics<SemanticDiagnostic>,
    generic_params: Vec<GenericParam>,
}

// Selectors.
/// Query implementation of [crate::db::SemanticGroup::extern_type_declaration_diagnostics].
pub fn extern_type_declaration_diagnostics(
    db: &dyn SemanticGroup,
    extern_type_id: ExternTypeId,
) -> Diagnostics<SemanticDiagnostic> {
    db.priv_extern_type_declaration_data(extern_type_id)
        .map(|data| data.diagnostics)
        .unwrap_or_default()
}
/// Query implementation of [crate::db::SemanticGroup::extern_type_declaration_generic_params].
pub fn extern_type_declaration_generic_params(
    db: &dyn SemanticGroup,
    extern_type_id: ExternTypeId,
) -> Maybe<Vec<GenericParam>> {
    Ok(db.priv_extern_type_declaration_data(extern_type_id)?.generic_params)
}

// Computation.
/// Query implementation of [crate::db::SemanticGroup::priv_extern_type_declaration_data].
pub fn priv_extern_type_declaration_data(
    db: &dyn SemanticGroup,
    extern_type_id: ExternTypeId,
) -> Maybe<ExternTypeDeclarationData> {
    let module_file_id = extern_type_id.module_file_id(db.upcast());
    let mut diagnostics = SemanticDiagnostics::new(module_file_id);
    let module_extern_types = db.module_extern_types(module_file_id.0)?;
    let type_syntax = module_extern_types.get(&extern_type_id).to_maybe()?;

    let mut resolver = Resolver::new(db, module_file_id);
    let generic_params = semantic_generic_params(
        db,
        &mut diagnostics,
        &mut resolver,
        module_file_id,
        &type_syntax.generic_params(db.upcast()),
        false,
    )?;
    if let Some(param) = generic_params.iter().find(|param| param.kind() == GenericKind::Impl) {
        diagnostics.report_by_ptr(
            param.stable_ptr(db.upcast()).untyped(),
            ExternItemWithImplGenericsNotSupported,
        );
    }

    // Check fully resolved.
    if let Some((_stable_ptr, inference_err)) = resolver.inference().finalize() {
        // TODO: Better location.
        inference_err.report(&mut diagnostics, type_syntax.stable_ptr().untyped());
    }
    let generic_params = resolver
        .inference()
        .rewrite(generic_params)
        .map_err(|err| err.report(&mut diagnostics, type_syntax.stable_ptr().untyped()))?;

    Ok(ExternTypeDeclarationData { diagnostics: diagnostics.build(), generic_params })
}
