//! Font library file storage and rules served through `desktop_shell.proto`
//! from `taigi_desktop_storage::font_library`.
//!
//! Answered in `Shell::serve` before taking `Shell::session` (and without
//! gating on `Configure` or mutating `Shell::settings`) so storage helpers and
//! tests behave identically to `taigi-desktop-storage` at any point in process
//! startup.

use std::path::Path;

use taigi_desktop_storage::{
    self as storage, ImportError, ALLOWED_EXTENSIONS, FONTS_FOLDER_NAME, MAX_FILE_SIZE,
    MAX_NAME_ATTEMPTS,
};

use crate::proto::{
    desktop_request, desktop_response, font_library_error, font_library_reply,
    font_library_request, CopyInFontResult, FontLibraryAck, FontLibraryError, FontLibraryReply,
    FontLibraryRequest, FontLibraryRules, SanitizedFontComponent, SanitizedFontStem, StoredFonts,
};
use crate::runtime::Refusal;

/// Handles `FontLibrary` requests; `None` when `request` belongs to another
/// handler (`key_rules`, `user_data`, `dictionary_search`, or `Shell::dispatch`).
pub(crate) fn answer(
    request: &desktop_request::Request,
) -> Option<Result<desktop_response::Reply, Refusal>> {
    match request {
        desktop_request::Request::FontLibrary(req) => {
            Some(serve_font_library(req).map(desktop_response::Reply::FontLibrary))
        }
        _ => None,
    }
}

fn serve_font_library(request: &FontLibraryRequest) -> Result<FontLibraryReply, Refusal> {
    use font_library_reply::Result as ReplyResult;
    use font_library_request::Method;

    let Some(method) = &request.method else {
        return Err(Refusal::Missing("font_library.method"));
    };

    match method {
        Method::Rules(_) => Ok(ok_reply(ReplyResult::Rules(rules()))),
        Method::CopyIn(req) => {
            if req.directory.is_empty() {
                return Err(Refusal::Missing("directory"));
            }
            if req.source.is_empty() {
                return Err(Refusal::Missing("source"));
            }
            match storage::copy_in(Path::new(&req.directory), Path::new(&req.source)) {
                Ok(file_name) => Ok(ok_reply(ReplyResult::Copied(CopyInFontResult {
                    file_name,
                }))),
                Err(error) => Ok(error_reply(to_proto_import_error(error))),
            }
        }
        Method::RemoveStored(req) => {
            if req.directory.is_empty() {
                return Err(Refusal::Missing("directory"));
            }
            let Some(file_name) = storage::sanitized_component(&req.file_name) else {
                return Ok(error_reply(font_library_error::Kind::InvalidFileName(true)));
            };
            match storage::remove_stored(Path::new(&req.directory), file_name) {
                Ok(()) => Ok(ok_reply(ReplyResult::Ack(FontLibraryAck {}))),
                Err(error) => Ok(error_reply(font_library_error::Kind::NotRemoved(
                    error.to_string(),
                ))),
            }
        }
        Method::ListStored(req) => {
            if req.directory.is_empty() {
                return Err(Refusal::Missing("directory"));
            }
            Ok(ok_reply(ReplyResult::StoredFonts(StoredFonts {
                file_names: storage::stored_file_names(Path::new(&req.directory)),
            })))
        }
        Method::SanitizedStem(req) => Ok(ok_reply(ReplyResult::SanitizedStem(SanitizedFontStem {
            stem: storage::sanitized_stem(&req.name),
        }))),
        Method::SanitizedComponent(req) => Ok(ok_reply(ReplyResult::SanitizedComponent(
            SanitizedFontComponent {
                file_name: storage::sanitized_component(&req.file_name).map(str::to_owned),
            },
        ))),
    }
}

fn ok_reply(result: font_library_reply::Result) -> FontLibraryReply {
    FontLibraryReply {
        error: None,
        result: Some(result),
    }
}

fn error_reply(kind: font_library_error::Kind) -> FontLibraryReply {
    FontLibraryReply {
        error: Some(FontLibraryError { kind: Some(kind) }),
        result: None,
    }
}

fn to_proto_import_error(error: ImportError) -> font_library_error::Kind {
    match error {
        ImportError::UnsupportedFileType(ext) => font_library_error::Kind::UnsupportedFileType(ext),
        ImportError::NotARegularFile => font_library_error::Kind::NotARegularFile(true),
        ImportError::TooLarge(bytes) => font_library_error::Kind::TooLargeBytes(bytes),
        ImportError::Unreadable(error) => font_library_error::Kind::Unreadable(error.to_string()),
        ImportError::NotCopied(error) => font_library_error::Kind::NotCopied(error.to_string()),
        ImportError::NoAvailableName => font_library_error::Kind::NoAvailableName(true),
        ImportError::Directory(error) => font_library_error::Kind::NotCopied(error.to_string()),
    }
}

pub(crate) fn rules() -> FontLibraryRules {
    FontLibraryRules {
        allowed_extensions: ALLOWED_EXTENSIONS
            .iter()
            .map(|ext| (*ext).to_owned())
            .collect(),
        max_file_size: MAX_FILE_SIZE,
        max_name_attempts: u32::try_from(MAX_NAME_ATTEMPTS).unwrap_or(u32::MAX),
        fonts_folder_name: FONTS_FOLDER_NAME.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{
        CopyInFontRequest, FontLibraryRulesRequest, ListStoredFontsRequest,
        RemoveStoredFontRequest, SanitizedFontComponentRequest, SanitizedFontStemRequest,
    };

    #[test]
    fn rules_match_desktop_storage_constants() {
        let reply = rules();
        assert_eq!(reply.allowed_extensions, vec!["ttf", "otf", "ttc"]);
        assert_eq!(reply.max_file_size, 64 * 1024 * 1024);
        assert_eq!(reply.max_name_attempts, 100);
        assert_eq!(reply.fonts_folder_name, "Fonts");
    }

    #[test]
    fn import_error_maps_every_variant() {
        assert_eq!(
            to_proto_import_error(ImportError::UnsupportedFileType("txt".to_owned())),
            font_library_error::Kind::UnsupportedFileType("txt".to_owned())
        );
        assert_eq!(
            to_proto_import_error(ImportError::NotARegularFile),
            font_library_error::Kind::NotARegularFile(true)
        );
        assert_eq!(
            to_proto_import_error(ImportError::TooLarge(64 * 1024 * 1024 + 1)),
            font_library_error::Kind::TooLargeBytes(64 * 1024 * 1024 + 1)
        );
        assert_eq!(
            to_proto_import_error(ImportError::Unreadable(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such file"
            ))),
            font_library_error::Kind::Unreadable("no such file".to_owned())
        );
        assert_eq!(
            to_proto_import_error(ImportError::NotCopied(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied"
            ))),
            font_library_error::Kind::NotCopied("permission denied".to_owned())
        );
        assert_eq!(
            to_proto_import_error(ImportError::NoAvailableName),
            font_library_error::Kind::NoAvailableName(true)
        );
        assert_eq!(
            to_proto_import_error(ImportError::Directory(storage::DirectoryError::NoAppData)),
            font_library_error::Kind::NotCopied(storage::DirectoryError::NoAppData.to_string())
        );
    }

    #[test]
    fn missing_fields_are_refused() {
        assert_eq!(
            serve_font_library(&FontLibraryRequest { method: None }),
            Err(Refusal::Missing("font_library.method"))
        );
        assert_eq!(
            serve_font_library(&FontLibraryRequest {
                method: Some(font_library_request::Method::CopyIn(CopyInFontRequest {
                    directory: String::new(),
                    source: "/tmp/font.ttf".to_owned(),
                })),
            }),
            Err(Refusal::Missing("directory"))
        );
        assert_eq!(
            serve_font_library(&FontLibraryRequest {
                method: Some(font_library_request::Method::CopyIn(CopyInFontRequest {
                    directory: "/tmp/Fonts".to_owned(),
                    source: String::new(),
                })),
            }),
            Err(Refusal::Missing("source"))
        );
        assert_eq!(
            serve_font_library(&FontLibraryRequest {
                method: Some(font_library_request::Method::RemoveStored(
                    RemoveStoredFontRequest {
                        directory: String::new(),
                        file_name: "font.ttf".to_owned(),
                    },
                )),
            }),
            Err(Refusal::Missing("directory"))
        );
        assert_eq!(
            serve_font_library(&FontLibraryRequest {
                method: Some(font_library_request::Method::ListStored(
                    ListStoredFontsRequest {
                        directory: String::new(),
                    },
                )),
            }),
            Err(Refusal::Missing("directory"))
        );
    }

    #[test]
    fn empty_and_traversal_file_names_do_not_refuse_request() {
        let dir = tempfile::tempdir().expect("temp dir");
        for bad_name in [
            "",
            ".",
            "..",
            "../keep.ttf",
            "./keep.ttf",
            "sub/keep.ttf",
            "..\\keep.ttf",
        ] {
            let reply = serve_font_library(&FontLibraryRequest {
                method: Some(font_library_request::Method::RemoveStored(
                    RemoveStoredFontRequest {
                        directory: dir.path().display().to_string(),
                        file_name: bad_name.to_owned(),
                    },
                )),
            })
            .expect("invalid file name maps to FontLibraryReply.error");
            assert_eq!(
                reply.error,
                Some(FontLibraryError {
                    kind: Some(font_library_error::Kind::InvalidFileName(true)),
                }),
                "{bad_name:?}"
            );
            assert_eq!(reply.result, None);

            let component_reply = serve_font_library(&FontLibraryRequest {
                method: Some(font_library_request::Method::SanitizedComponent(
                    SanitizedFontComponentRequest {
                        file_name: bad_name.to_owned(),
                    },
                )),
            })
            .expect("sanitized component answers None");
            assert_eq!(
                component_reply.result,
                Some(font_library_reply::Result::SanitizedComponent(
                    SanitizedFontComponent { file_name: None }
                )),
                "{bad_name:?}"
            );
        }

        let valid_component = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::SanitizedComponent(
                SanitizedFontComponentRequest {
                    file_name: "mine.ttf".to_owned(),
                },
            )),
        })
        .unwrap();
        assert_eq!(
            valid_component.result,
            Some(font_library_reply::Result::SanitizedComponent(
                SanitizedFontComponent {
                    file_name: Some("mine.ttf".to_owned()),
                }
            ))
        );

        let empty_stem = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::SanitizedStem(
                SanitizedFontStemRequest {
                    name: String::new(),
                },
            )),
        })
        .unwrap();
        assert_eq!(
            empty_stem.result,
            Some(font_library_reply::Result::SanitizedStem(
                SanitizedFontStem {
                    stem: "typeface".to_owned(),
                }
            ))
        );
    }

    #[test]
    fn copy_list_and_remove_round_trip_in_temp_dir() {
        let dir = tempfile::tempdir().expect("temp dir");
        let library = dir.path().join("Fonts");
        std::fs::create_dir(&library).unwrap();
        let source_dir = dir.path().join("src");
        std::fs::create_dir(&source_dir).unwrap();
        let source = source_dir.join("Iansui.ttf");
        std::fs::write(&source, b"fake-font-bytes").unwrap();

        let rules_reply = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::Rules(
                FontLibraryRulesRequest {},
            )),
        })
        .unwrap();
        assert_eq!(
            rules_reply.result,
            Some(font_library_reply::Result::Rules(rules()))
        );

        let first = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::CopyIn(CopyInFontRequest {
                directory: library.display().to_string(),
                source: source.display().to_string(),
            })),
        })
        .unwrap();
        assert_eq!(
            first.result,
            Some(font_library_reply::Result::Copied(CopyInFontResult {
                file_name: "Iansui.ttf".to_owned(),
            }))
        );

        let second = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::CopyIn(CopyInFontRequest {
                directory: library.display().to_string(),
                source: source.display().to_string(),
            })),
        })
        .unwrap();
        assert_eq!(
            second.result,
            Some(font_library_reply::Result::Copied(CopyInFontResult {
                file_name: "Iansui-2.ttf".to_owned(),
            }))
        );

        let listed = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::ListStored(
                ListStoredFontsRequest {
                    directory: library.display().to_string(),
                },
            )),
        })
        .unwrap();
        assert_eq!(
            listed.result,
            Some(font_library_reply::Result::StoredFonts(StoredFonts {
                file_names: vec!["Iansui-2.ttf".to_owned(), "Iansui.ttf".to_owned()],
            }))
        );

        let removed = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::RemoveStored(
                RemoveStoredFontRequest {
                    directory: library.display().to_string(),
                    file_name: "Iansui.ttf".to_owned(),
                },
            )),
        })
        .unwrap();
        assert_eq!(
            removed.result,
            Some(font_library_reply::Result::Ack(FontLibraryAck {}))
        );

        let missing_remove = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::RemoveStored(
                RemoveStoredFontRequest {
                    directory: library.display().to_string(),
                    file_name: "Iansui.ttf".to_owned(),
                },
            )),
        })
        .unwrap();
        assert!(matches!(
            missing_remove.error,
            Some(FontLibraryError {
                kind: Some(font_library_error::Kind::NotRemoved(_)),
            })
        ));

        let invalid_dir_remove = serve_font_library(&FontLibraryRequest {
            method: Some(font_library_request::Method::RemoveStored(
                RemoveStoredFontRequest {
                    directory: "/tmp/bad\0dir".to_owned(),
                    file_name: "Iansui.ttf".to_owned(),
                },
            )),
        })
        .unwrap();
        assert!(matches!(
            invalid_dir_remove.error,
            Some(FontLibraryError {
                kind: Some(font_library_error::Kind::NotRemoved(_)),
            })
        ));
    }
}
