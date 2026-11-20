//! Único punto de traducción de `ApplicationError` a `tonic::Status` con detalles `google.rpc`.

use std::collections::HashMap;

use tonic::{Code, Status};
use tonic_types::{ErrorDetails, StatusExt};

use crate::application::ApplicationError;

const ERROR_DOMAIN: &str = "synapse.sync.v1";

/// Traduce un `ApplicationError` al `tonic::Status` definido en la tabla de errores del diseño.
pub fn to_status(err: ApplicationError) -> Status {
    let message = err.to_string();
    match err {
        ApplicationError::Unauthenticated(_) => Status::unauthenticated(message),
        ApplicationError::PermissionDenied(_) => Status::permission_denied(message),
        ApplicationError::Unavailable(_) => Status::unavailable(message),
        ApplicationError::NotFound(_) => Status::not_found(message),
        ApplicationError::InvalidArgument { field, reason } => {
            let mut details = ErrorDetails::new();
            details.add_bad_request_violation(field, reason);
            Status::with_error_details(Code::InvalidArgument, message, details)
        }
        ApplicationError::MissingContent { missing_hashes } => {
            let mut details = ErrorDetails::new();
            for hash in missing_hashes {
                details.add_precondition_failure_violation(
                    "MISSING_CONTENT",
                    hash,
                    "contenido no encontrado en el almacén",
                );
            }
            Status::with_error_details(Code::FailedPrecondition, message, details)
        }
        ApplicationError::RootMismatch => {
            let mut details = ErrorDetails::new();
            details.set_error_info("ROOT_MISMATCH", ERROR_DOMAIN, HashMap::new());
            Status::with_error_details(Code::FailedPrecondition, message, details)
        }
        ApplicationError::StaleBase { current_root_hash } => {
            let mut details = ErrorDetails::new();
            let mut metadata = HashMap::new();
            metadata.insert("current_root_hash".into(), current_root_hash);
            details.set_error_info("STALE_BASE", ERROR_DOMAIN, metadata);
            Status::with_error_details(Code::Aborted, message, details)
        }
        ApplicationError::ResourceExhausted(_) => Status::resource_exhausted(message),
        ApplicationError::Internal(_) => Status::internal(message),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn mapea_codigos_simples() {
        assert_eq!(
            to_status(ApplicationError::Unauthenticated("sin token".into())).code(),
            Code::Unauthenticated
        );
        assert_eq!(
            to_status(ApplicationError::PermissionDenied("no permitida".into())).code(),
            Code::PermissionDenied
        );
        assert_eq!(
            to_status(ApplicationError::Unavailable("jwks".into())).code(),
            Code::Unavailable
        );
        assert_eq!(
            to_status(ApplicationError::NotFound("bóveda".into())).code(),
            Code::NotFound
        );
        assert_eq!(
            to_status(ApplicationError::ResourceExhausted("tamaño".into())).code(),
            Code::ResourceExhausted
        );
        assert_eq!(
            to_status(ApplicationError::Internal("fallo".into())).code(),
            Code::Internal
        );
    }

    #[test]
    fn mapea_invalid_argument_con_bad_request() {
        let status = to_status(ApplicationError::InvalidArgument {
            field: "content_hash".into(),
            reason: "no coincide".into(),
        });
        assert_eq!(status.code(), Code::InvalidArgument);
        let details = status.get_error_details();
        let bad_req = details.bad_request().unwrap();
        assert_eq!(bad_req.field_violations.len(), 1);
        assert_eq!(bad_req.field_violations[0].field, "content_hash");
    }

    #[test]
    fn mapea_missing_content_con_precondition_failure() {
        let status = to_status(ApplicationError::MissingContent {
            missing_hashes: vec!["aaa".into(), "bbb".into()],
        });
        assert_eq!(status.code(), Code::FailedPrecondition);
        let details = status.get_error_details();
        let pre = details.precondition_failure().unwrap();
        assert_eq!(pre.violations.len(), 2);
        assert_eq!(pre.violations[0].r#type, "MISSING_CONTENT");
        assert_eq!(pre.violations[0].subject, "aaa");
        assert_eq!(pre.violations[1].subject, "bbb");
    }

    #[test]
    fn mapea_root_mismatch_y_stale_base_con_error_info() {
        let mismatch = to_status(ApplicationError::RootMismatch);
        assert_eq!(mismatch.code(), Code::FailedPrecondition);
        let info = mismatch.get_error_details();
        assert_eq!(info.error_info().unwrap().reason, "ROOT_MISMATCH");

        let stale = to_status(ApplicationError::StaleBase {
            current_root_hash: "abc123".into(),
        });
        assert_eq!(stale.code(), Code::Aborted);
        let info = stale.get_error_details();
        let error_info = info.error_info().unwrap();
        assert_eq!(error_info.reason, "STALE_BASE");
        assert_eq!(
            error_info
                .metadata
                .get("current_root_hash")
                .map(String::as_str),
            Some("abc123")
        );
    }
}
