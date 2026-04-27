//! Android JNI entry point. Thin wrapper around `phonetics::process_request`
//! plus a logger callback that bounces back into the JVM via a cached
//! `JavaVM` + `GlobalRef` to the `RustEngineBridge` class.
//!
//! Per `docs/engine/ffi-safety.md` §2, every exported `extern "system"` body is
//! wrapped in `catch_unwind`. Per plan v3 §B3 + plan v4 §R3-H2 the JNI body
//! length-checks the `jbyteArray` BEFORE copying into a Rust `Vec<u8>`, so an
//! oversized payload is rejected without the matching allocation.

use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JStaticMethodID, JValue};
use jni::signature::{Primitive, ReturnType};
use jni::sys::{jbyteArray, jint};
use jni::{JNIEnv, JavaVM};
use prost::Message;
use protos::engine::{ErrorCode, Response};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Once, OnceLock};

/// 2 MB cap on the request byte buffer. See `engine/swift-ffi/src/lib.rs` for
/// the matching iOS constant; both crates must agree.
pub const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;

const BRIDGE_CLASS: &str = "com/siansiansu/taigikeyboard/engine/RustEngineBridge";
const DISPATCH_METHOD: &str = "dispatchLog";
const DISPATCH_SIG: &str = "(ILjava/lang/String;Ljava/lang/String;)V";

// MARK: - JNI exports

/// `external fun processRequestBytes(bytes: ByteArray): ByteArray` declared on
/// `com.siansiansu.taigikeyboard.engine.RustEngineBridge`.
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_processRequestBytes<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    bytes: JByteArray<'local>,
) -> jbyteArray {
    let result = catch_unwind(AssertUnwindSafe(|| -> Vec<u8> {
        // Pre-copy length check per plan v4 R3-H2 — avoids allocating a large
        // Vec<u8> for input we are about to reject.
        let len = match env.get_array_length(&bytes) {
            Ok(l) => l as usize,
            Err(_) => return encode_error(0, ErrorCode::FailParse, 0),
        };
        if len > MAX_REQUEST_BYTES {
            // JUSTIFICATION: mapped to FAIL_INVARIANT (not FAIL_PARSE) — bytes
            // may be wire-valid; the engine invariant violated is "request
            // size ≤ MAX_REQUEST_BYTES". Adding FAIL_SIZE would renumber proto
            // reserved fields.
            return encode_error(0, ErrorCode::FailInvariant, 0);
        }
        let bytes_vec = match env.convert_byte_array(&bytes) {
            Ok(v) => v,
            Err(_) => return encode_error(0, ErrorCode::FailParse, 0),
        };
        phonetics::process_request(&bytes_vec)
    }));
    let response_bytes = result.unwrap_or_else(|_| encode_error(0, ErrorCode::FailInternal, 0));
    match env.byte_array_from_slice(&response_bytes) {
        Ok(arr) => arr.into_raw(),
        Err(_) => JObject::null().into_raw() as jbyteArray,
    }
}

/// `external fun registerLogger(): Unit`. Caches `JavaVM` + a `GlobalRef` to
/// the bridge class + the `dispatchLog` static method ID, then installs the
/// Rust `log` adapter.
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_registerLogger<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let vm = match env.get_java_vm() {
            Ok(v) => v,
            Err(_) => return,
        };
        let class = match env.find_class(BRIDGE_CLASS) {
            Ok(c) => c,
            Err(_) => return,
        };
        let class_global = match env.new_global_ref(&class) {
            Ok(g) => g,
            Err(_) => return,
        };
        let method_id = match env.get_static_method_id(&class, DISPATCH_METHOD, DISPATCH_SIG) {
            Ok(id) => id,
            Err(_) => return,
        };
        // Invariant: cache only `JavaVM` + `GlobalRef`. `JNIEnv` is
        // per-thread and would dangle if read from another thread.
        let _ = LOGGER_DISPATCH.set(LoggerDispatch {
            vm,
            class_global,
            method_id_bits: jmethod_id_to_bits(method_id),
        });
        SET_LOGGER.call_once(|| {
            let _ = log::set_logger(&PLATFORM_LOGGER);
            // Default to Warn — see the matching note in
            // `engine/swift-ffi/src/lib.rs::install_logger_sink` for
            // rationale. Kotlin bridges call `setLogLevel(4)` in
            // BuildConfig.DEBUG to opt into `Debug`.
            log::set_max_level(log::LevelFilter::Warn);
        });
    }));
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
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    level: jni::sys::jint,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
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
    }));
}

/// T1 panic injector. Available only when the `panic-injector` feature is
/// enabled (dev `.so`). Release builds omit this symbol; verified by `nm` in
/// `build-android-libs.sh`.
#[cfg(feature = "panic-injector")]
#[no_mangle]
pub extern "system" fn Java_com_siansiansu_taigikeyboard_engine_RustEngineBridge_panicForTest<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jbyteArray {
    let result = catch_unwind(AssertUnwindSafe(|| -> Vec<u8> {
        panic!("intentional T1 panic at android-jni boundary");
    }));
    let response_bytes = result.unwrap_or_else(|_| encode_error(0, ErrorCode::FailInternal, 0));
    match env.byte_array_from_slice(&response_bytes) {
        Ok(arr) => arr.into_raw(),
        Err(_) => JObject::null().into_raw() as jbyteArray,
    }
}

// MARK: - Logger glue

struct LoggerDispatch {
    vm: JavaVM,
    class_global: GlobalRef,
    /// `JStaticMethodID` borrows lifetime from the `JNIEnv` it was looked up
    /// on, so we round-trip through the raw `jmethodID` pointer bits and
    /// reconstruct on use. The class is kept alive by `class_global`.
    method_id_bits: usize,
}

// SAFETY: `JavaVM` and `GlobalRef` are documented as thread-safe by the `jni`
// crate. The raw `jmethodID` is process-wide stable for the lifetime of the
// loaded class, which `class_global` keeps alive.
unsafe impl Send for LoggerDispatch {}
unsafe impl Sync for LoggerDispatch {}

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
        // Per Codex round-2 H1: attach the current thread on every callback;
        // never cache the JNIEnv. AttachGuard auto-detaches on drop.
        let Ok(mut env) = dispatch.vm.attach_current_thread() else {
            return;
        };
        let level = level_to_jint(record.level());
        let Ok(tag) = env.new_string(record.target()) else {
            return;
        };
        let Ok(msg) = env.new_string(format!("{}", record.args())) else {
            return;
        };
        // SAFETY: `dispatch.method_id_bits` was produced from a `JMethodID`
        // obtained at registration time, on a class kept alive by
        // `class_global`. The signature `(ILjava/lang/String;Ljava/lang/String;)V`
        // matches the `tag` + `msg` arguments below.
        let method_id = unsafe { bits_to_jmethod_id(dispatch.method_id_bits) };
        let _ = unsafe {
            env.call_static_method_unchecked(
                &dispatch.class_global,
                method_id,
                ReturnType::Primitive(Primitive::Void),
                &[
                    JValue::from(level).as_jni(),
                    JValue::from(&tag).as_jni(),
                    JValue::from(&msg).as_jni(),
                ],
            )
        };
        // Clear any pending Java exception so the JVM is not poisoned.
        if let Ok(true) = env.exception_check() {
            let _ = env.exception_clear();
        }
    }

    fn flush(&self) {}
}

fn level_to_jint(level: log::Level) -> jint {
    match level {
        log::Level::Error => 0,
        log::Level::Warn => 1,
        log::Level::Info => 2,
        log::Level::Debug => 3,
        log::Level::Trace => 4,
    }
}

fn jmethod_id_to_bits(id: JStaticMethodID) -> usize {
    id.into_raw() as usize
}

/// SAFETY contract is documented at the call-site in `PlatformLogger::log`.
unsafe fn bits_to_jmethod_id(bits: usize) -> JStaticMethodID {
    JStaticMethodID::from_raw(bits as *mut _)
}

fn encode_error(id: u32, code: ErrorCode, generation: u64) -> Vec<u8> {
    let response = Response {
        id,
        error: code as i32,
        generation,
        payload: None,
    };
    let mut buf = Vec::with_capacity(response.encoded_len());
    // JUSTIFICATION: encode into Vec<u8> never fails — prost::EncodeError
    // fires only when the target buffer is too small; Vec grows.
    response
        .encode(&mut buf)
        .expect("prost encode into Vec<u8> never fails");
    buf
}
