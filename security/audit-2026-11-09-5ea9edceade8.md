# Audit

Revision: `5ea9edceade89940f61ffc5be59d4211558ccff3` on `main`.

- `Cargo.toml` reviewed with no findings.
- `context/forms.md` reviewed with no findings.
- `context/getting-started.md` reviewed with no findings.
- `design/form-content.md` reviewed with no findings.
- `design/urls.md` reviewed with no findings.
- `readme.md` reviewed with no findings.
- `releases.md` reviewed with no findings.
- `src/form_error.rs` reviewed with no findings.
- `src/form_event.rs` reviewed with no findings.
- `src/form_limits.rs` reviewed with no findings.
- `src/form_name.rs` reviewed with no findings.
- `src/form_pairs.rs` reviewed with no findings.
- `src/form_parser.rs` reviewed with findings:
  - low: values have no length limit tighter than the size limit by default, so one value can be 2 MiB; rejected, a value is never held, only scanned and decoded piece by piece, and a consumer that knows what a value is for applies a tighter limit (`// SECURITY:` at the value state in `FormParser::step`).
  - low: decoded names and values can contain NUL, CR, and LF, which could inject into another syntax such as a header or a log line; rejected, they are data that the parser does not interpret, and encoding them for another syntax belongs to the consumer that writes it (`// SECURITY:` at `decode`).
- `src/form_path.rs` reviewed with no findings.
- `src/lib.rs` reviewed with no findings.
- `tests/form_parser.rs` reviewed with no findings.
- `tests/form_path.rs` reviewed with no findings.
