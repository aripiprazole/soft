use miette::NamedSource;

use crate::{SrcPos, semantic::SemanticError};

pub struct Generator {
    pub text_source: NamedSource,
    pub location: SrcPos,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[diagnostic(url(docsrs))]
#[error("soft error")]
pub struct SoftError {
    #[source_code]
    pub text_source: NamedSource,

    #[related]
    pub related: Vec<InnerError>,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug, Clone)]
#[diagnostic(url(docsrs))]
pub enum InnerError {
    #[error("semantic error: {0}")]
    #[diagnostic(code(soft::semantic))]
    SemanticError(SemanticError),
}
