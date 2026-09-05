//! Android JNI entry point. Thin wrapper around `dispatch::process_request`
//! plus a logger callback that bounces back into the JVM via a cached
//! `JavaVM` + `Global<JClass<'static>>` reference to the `RustEngineBridge`
//! class.
//!
//! Per `docs/engine/ffi-safety.md` §2, every exported `extern "system"` body
//! is run inside [`EnvUnowned::with_env`], which wraps the closure in
//! [`std::panic::catch_unwind`] so panics never unwind across the JNI
//! boundary. Per plan v3 §B3 + plan v4 §R3-H2 the JNI body length-checks the
//! `jbyteArray` BEFORE copying into a Rust `Vec<u8>`, so an oversized payload
//! is rejected without the matching allocation.

use dispatch::{encode_error, log_level_to_byte, MAX_REQUEST_BYTES};
use jni::objects::{Global, JByteArray, JClass, JObject, JStaticMethodID, JValue};
use jni::signature::{MethodSignature, Primitive, ReturnType};
use jni::strings::JNIStr;
use jni::sys::{jbyteArray, jint};
use jni::{jni_sig, jni_str, Env, EnvUnowned, JavaVM, Outcome};
use protos::engine::ErrorCode;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Once, OnceLock};

// jni 0.22 requires `AsRef<JNIStr>` and `AsRef<MethodSignature>` for class /
// method / signature arguments. The `jni_str!` and `jni_sig!` macros perform
// MUTF-8 / signature validation at compile time.
const BRIDGE_CLASS: &JNIStr = jni_str!("com/siansiansu/taigikeyboard/engine/RustEngineBridge");
const DISPATCH_METHOD: &JNIStr = jni_str!("dispatchLog");
const DISPATCH_SIG: MethodSignature = jni_sig!("(ILjava/lang/String;Ljava/lang/String;)V");

// MARK: - JNI exports

/// `external fun processRequestBytes(bytes: ByteArray): ByteArray` declared on
/// `com.siansiansu.taigikeyboard.engine.RustEngineBridge`.
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_processRequestBytes<
    'local,
>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    bytes: JByteArray<'local>,
) -> jbyteArray {
    let outcome = unowned
        .with_env(|env| -> jni::errors::Result<jbyteArray> {
            // Pre-copy length check per plan v4 R3-H2 — avoids allocating a
            // large Vec<u8> for input we are about to reject. A failed
            // length probe is mapped to FAIL_PARSE rather than propagated
            // because the caller-visible contract is "return an encoded
            // Response, never throw".
            let len = match bytes.len(env) {
                Ok(l) => l,
                Err(_) => return encode_error_to_jarray(env, ErrorCode::FailParse),
            };
            if len > MAX_REQUEST_BYTES {
                // JUSTIFICATION: mapped to FAIL_INVARIANT (not FAIL_PARSE) —
                // bytes may be wire-valid; the engine invariant violated is
                // "request size ≤ MAX_REQUEST_BYTES". Adding FAIL_SIZE would
                // renumber proto reserved fields.
                return encode_error_to_jarray(env, ErrorCode::FailInvariant);
            }
            let bytes_vec = match env.convert_byte_array(&bytes) {
                Ok(v) => v,
                Err(_) => return encode_error_to_jarray(env, ErrorCode::FailParse),
            };
            let response_bytes = dispatch::process_request(&bytes_vec);
            Ok(env.byte_array_from_slice(&response_bytes)?.into_raw())
        })
        .into_outcome();

    match outcome {
        Outcome::Ok(arr) => arr,
        // Both Err (an unrecovered JNI failure such as byte_array_from_slice
        // running out of memory) and Panic flow back through one fresh
        // attachment to preserve the pre-0.22 contract: the Kotlin side always
        // sees a Response-encoded byte array, even on internal failure.
        Outcome::Err(_) | Outcome::Panic(_) => {
            encode_error_via_fresh_attach(&mut unowned, ErrorCode::FailInternal)
        }
    }
}

/// `external fun registerLogger(): Unit`. Caches `JavaVM` + a
/// `Global<JClass<'static>>` to the bridge class + the `dispatchLog` static
/// method ID, then installs the Rust `log` adapter.
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_registerLogger<
    'local,
>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    let _ = unowned
        .with_env(|env| -> jni::errors::Result<()> {
            let vm = env.get_java_vm()?;
            let class = env.find_class(BRIDGE_CLASS)?;
            let class_global: Global<JClass<'static>> = env.new_global_ref(&class)?;
            let method_id = env.get_static_method_id(&class, DISPATCH_METHOD, DISPATCH_SIG)?;
            // Invariant: cache only `JavaVM` + `Global<JClass<'static>>` +
            // method ID. `Env` is per-thread and would dangle if read from
            // another thread.
            let _ = LOGGER_DISPATCH.set(LoggerDispatch {
                vm,
                class_global,
                method_id,
            });
            SET_LOGGER.call_once(|| {
                let _ = log::set_logger(&PLATFORM_LOGGER);
                // Default to Warn — see the matching note in
                // `engine/swift-ffi/src/lib.rs::install_logger_sink` for
                // rationale. Kotlin bridges call `setLogLevel(4)` in
                // BuildConfig.DEBUG to opt into `Debug`.
                log::set_max_level(log::LevelFilter::Warn);
            });
            Ok(())
        })
        .into_outcome();
}

/// JNI mirror of swift-ffi `set_log_level`. Adjusts Rust `log::max_level`
/// at runtime so platform DEBUG builds can opt into `Debug` verbosity
/// without release builds paying the format cost.
///
/// Levels: 0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace; anything
/// else → `Off`.
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_setLogLevel<
    'local,
>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
    level: jni::sys::jint,
) {
    let _ = unowned
        .with_env(|_env| -> jni::errors::Result<()> {
            let filter = match level {
                1 => log::LevelFilter::Error,
                2 => log::LevelFilter::Warn,
                3 => log::LevelFilter::Info,
                4 => log::LevelFilter::Debug,
                5 => log::LevelFilter::Trace,
                _ => log::LevelFilter::Off,
            };
            log::set_max_level(filter);
            log::info!("rust log level set to {filter:?}");
            Ok(())
        })
        .into_outcome();
}

/// T1 panic injector. Available only when the `panic-injector` feature is
/// enabled (dev `.so`). Release builds omit this symbol; verified by `nm` in
/// `build-android-libs.sh`.
#[cfg(feature = "panic-injector")]
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_panicForTest<
    'local,
>(
    mut unowned: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jbyteArray {
    let outcome = unowned
        .with_env(|_env| -> jni::errors::Result<jbyteArray> {
            panic!("intentional T1 panic at android-jni boundary");
        })
        .into_outcome();
    match outcome {
        Outcome::Ok(arr) => arr,
        Outcome::Err(_) | Outcome::Panic(_) => {
            encode_error_via_fresh_attach(&mut unowned, ErrorCode::FailInternal)
        }
    }
}

// MARK: - Logger glue

struct LoggerDispatch {
    vm: JavaVM,
    /// Global reference to the bridge class. Keeps the class alive so the
    /// cached `method_id` stays valid for the lifetime of the loaded library.
    class_global: Global<JClass<'static>>,
    /// `JStaticMethodID` is `Copy + Send + Sync` (lifetime-free) since
    /// jni 0.22, so it can be stored directly without the prior `usize`
    /// round-trip.
    method_id: JStaticMethodID,
}

static LOGGER_DISPATCH: OnceLock<LoggerDispatch> = OnceLock::new();
static SET_LOGGER: Once = Once::new();
static PLATFORM_LOGGER: PlatformLogger = PlatformLogger;

struct PlatformLogger;

impl log::Log for PlatformLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let Some(dispatch) = LOGGER_DISPATCH.get() else {
            return;
        };
        // `JavaVM::attach_current_thread` does NOT wrap the closure in
        // catch_unwind in jni 0.22 — a panic there would unwind across the
        // JVM boundary and abort the process. Wrap explicitly so the logger
        // can never crash the host application.
        let _ = catch_unwind(AssertUnwindSafe(|| {
            // Per Codex round-2 H1: attach the current thread on every
            // callback; never cache the Env. The closure receives a fresh
            // `&mut Env` whose lifetime ends with the attach scope.
            let _ = dispatch
                .vm
                .attach_current_thread(|env| -> jni::errors::Result<()> {
                    let level = jint::from(log_level_to_byte(record.level()));
                    let tag = env.new_string(record.target())?;
                    let msg = env.new_string(format!("{}", record.args()))?;
                    // SAFETY: `dispatch.method_id` was looked up at registration
                    // time on a class kept alive by `class_global`. The signature
                    // `(ILjava/lang/String;Ljava/lang/String;)V` matches the
                    // `level` + `tag` + `msg` arguments below.
                    let _ = unsafe {
                        env.call_static_method_unchecked(
                            &dispatch.class_global,
                            dispatch.method_id,
                            ReturnType::Primitive(Primitive::Void),
                            &[
                                JValue::from(level).as_jni(),
                                JValue::from(&tag).as_jni(),
                                JValue::from(&msg).as_jni(),
                            ],
                        )
                    };
                    // Clear any pending Java exception so the JVM is not
                    // poisoned for the next caller.
                    if env.exception_check() {
                        env.exception_clear();
                    }
                    Ok(())
                });
        }));
    }

    fn flush(&self) {}
}

// MARK: - Error encoding helpers

/// Encode `code` as a `Response`, allocate a `JByteArray`, and return the raw
/// `jbyteArray`. Used inside `with_env` closures.
fn encode_error_to_jarray(env: &mut Env<'_>, code: ErrorCode) -> jni::errors::Result<jbyteArray> {
    let buf = encode_error(0, code, 0);
    Ok(env.byte_array_from_slice(&buf)?.into_raw())
}

/// Fallback path for `Outcome::Err` and `Outcome::Panic` on the JNI methods
/// that return `jbyteArray`: open a fresh `with_env` scope and re-encode the
/// error. If even that fails (OOM, JVM in a bad state), return `null`.
fn encode_error_via_fresh_attach(unowned: &mut EnvUnowned<'_>, code: ErrorCode) -> jbyteArray {
    let outcome = unowned
        .with_env(|env| -> jni::errors::Result<jbyteArray> { encode_error_to_jarray(env, code) })
        .into_outcome();
    match outcome {
        Outcome::Ok(arr) => arr,
        Outcome::Err(_) | Outcome::Panic(_) => JObject::null().into_raw() as jbyteArray,
    }
}
