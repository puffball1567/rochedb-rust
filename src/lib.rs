use std::error::Error as StdError;
use std::ffi::{CStr, CString, NulError};
use std::fmt;
use std::os::raw::{c_char, c_double, c_float, c_int, c_void};
use std::ptr;
use std::slice;
use std::str::FromStr;
use std::sync::Once;

pub const ROCHE_ABI_VERSION: i32 = 2;

const ROCHE_OK: c_int = 0;
const ROCHE_CODEC_RAW: c_int = 0;
const ROCHE_CODEC_JSON: c_int = 1;
const ROCHE_CODEC_NIF: c_int = 2;
const ROCHE_CODEC_BIF: c_int = 3;

static INIT: Once = Once::new();

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RocheId {
    pub parent: u64,
    pub epoch: u32,
    pub seq: u32,
    pub t_write: f64,
}

impl RocheId {
    pub fn new(parent: u64, epoch: u32, seq: u32, t_write: f64) -> Self {
        Self {
            parent,
            epoch,
            seq,
            t_write,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.parent == 0 && self.epoch == 0 && self.seq == 0 && self.t_write == 0.0
    }

    pub fn parse(value: &str) -> Result<Self, Error> {
        value.parse()
    }
}

impl fmt::Display for RocheId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.parent, self.epoch, self.seq, self.t_write
        )
    }
}

impl FromStr for RocheId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(':');
        let parent = parse_id_part::<u64>(&mut parts, "parent")?;
        let epoch = parse_id_part::<u32>(&mut parts, "epoch")?;
        let seq = parse_id_part::<u32>(&mut parts, "seq")?;
        let t_write = parse_id_part::<f64>(&mut parts, "t_write")?;
        if parts.next().is_some() {
            return Err(Error::new(
                ErrorKind::InvalidId,
                format!("invalid RocheDB id '{}': too many fields", value),
            ));
        }
        Ok(Self::new(parent, epoch, seq, t_write))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadCodec {
    Raw,
    Json,
    Nif,
    Bif,
}

impl PayloadCodec {
    fn as_c(self) -> c_int {
        match self {
            Self::Raw => ROCHE_CODEC_RAW,
            Self::Json => ROCHE_CODEC_JSON,
            Self::Nif => ROCHE_CODEC_NIF,
            Self::Bif => ROCHE_CODEC_BIF,
        }
    }

    fn from_c(value: c_int) -> Result<Self, Error> {
        match value {
            ROCHE_CODEC_RAW => Ok(Self::Raw),
            ROCHE_CODEC_JSON => Ok(Self::Json),
            ROCHE_CODEC_NIF => Ok(Self::Nif),
            ROCHE_CODEC_BIF => Ok(Self::Bif),
            _ => Err(Error::new(
                ErrorKind::Abi,
                format!("invalid RocheDB payload codec {}", value),
            )),
        }
    }
}

#[repr(C)]
struct RocheValue {
    data: *mut c_void,
    len: usize,
}

#[repr(C)]
struct RocheBatchResult {
    len: usize,
    values: *mut RocheValue,
}

#[repr(C)]
struct RocheHitRaw {
    id: RocheId,
    score: c_double,
    payload: *mut c_void,
    payload_len: usize,
}

#[repr(C)]
struct RocheRetrieveResultRaw {
    len: usize,
    hits: *mut RocheHitRaw,
    total_vectors: c_int,
    scanned: c_int,
    skipped_vectors: c_int,
    returned: c_int,
    rings_touched: c_int,
    payload_bytes: c_int,
    estimated_tokens: c_int,
    fanout_nodes: c_int,
    candidate_reduction: c_double,
}

#[link(name = "rochedb")]
extern "C" {
    fn roche_init();
    fn roche_abi_version() -> c_int;
    fn roche_last_error() -> *const c_char;
    fn roche_open(nodes: c_int) -> *mut c_void;
    fn roche_open_dir(nodes: c_int, dir: *const c_char) -> *mut c_void;
    fn roche_connect(peers: *const c_char) -> *mut c_void;
    fn roche_connect_auth(
        peers: *const c_char,
        username: *const c_char,
        password: *const c_char,
        auth_token: *const c_char,
        secret_key: *const c_char,
        galaxy: *const c_char,
    ) -> *mut c_void;
    fn roche_close(db: *mut c_void);
    fn roche_free(p: *mut c_void);
    fn roche_now(db: *mut c_void) -> c_double;
    fn roche_advance(db: *mut c_void, dt: c_double);
    fn roche_ring_configure(db: *mut c_void, ring: *const c_char, period: c_double) -> c_int;
    fn roche_set_galaxy_description(db: *mut c_void, description: *const c_char) -> c_int;
    fn roche_set_ring_description(
        db: *mut c_void,
        ring: *const c_char,
        description: *const c_char,
    ) -> c_int;
    fn roche_put(
        db: *mut c_void,
        ring: *const c_char,
        data: *const c_void,
        len: usize,
        out_id: *mut RocheId,
    ) -> c_int;
    fn roche_put_codec(
        db: *mut c_void,
        ring: *const c_char,
        data: *const c_void,
        len: usize,
        codec: c_int,
        out_id: *mut RocheId,
    ) -> c_int;
    fn roche_put_vec(
        db: *mut c_void,
        ring: *const c_char,
        data: *const c_void,
        len: usize,
        vec: *const c_float,
        vec_len: usize,
        out_id: *mut RocheId,
    ) -> c_int;
    fn roche_put_vec_codec(
        db: *mut c_void,
        ring: *const c_char,
        data: *const c_void,
        len: usize,
        codec: c_int,
        vec: *const c_float,
        vec_len: usize,
        out_id: *mut RocheId,
    ) -> c_int;
    fn roche_get(db: *mut c_void, id: RocheId, out_len: *mut usize) -> *mut c_void;
    fn roche_get_codec(
        db: *mut c_void,
        id: RocheId,
        out_len: *mut usize,
        out_codec: *mut c_int,
    ) -> *mut c_void;
    fn roche_batch_get(
        db: *mut c_void,
        ids: *const RocheId,
        ids_len: usize,
    ) -> *mut RocheBatchResult;
    fn roche_batch_get_free(r: *mut RocheBatchResult);
    fn roche_query(
        db: *mut c_void,
        id: RocheId,
        selection: *const c_char,
        out_len: *mut usize,
    ) -> *mut c_void;
    fn roche_read_ring_json(
        db: *mut c_void,
        ring: *const c_char,
        filter_json: *const c_char,
        selection: *const c_char,
        limit: c_int,
        cursor: *const c_char,
        pagination: c_int,
        page: c_int,
        page_limit: c_int,
        sort_field: *const c_char,
        sort_desc: c_int,
        out_len: *mut usize,
    ) -> *mut c_void;
    fn roche_retrieve(
        db: *mut c_void,
        vec: *const c_float,
        vec_len: usize,
        ring: *const c_char,
        budget: c_int,
        top_rings: c_int,
        focus: c_int,
    ) -> *mut RocheRetrieveResultRaw;
    fn roche_retrieve_free(r: *mut RocheRetrieveResultRaw);
    fn roche_atlas(
        db: *mut c_void,
        query_vec: *const c_float,
        query_vec_len: usize,
        max_centroid_dims: c_int,
        out_len: *mut usize,
    ) -> *mut c_void;
    fn roche_locate(db: *mut c_void, id: RocheId, at: c_double) -> c_int;
    fn roche_next_visit(db: *mut c_void, id: RocheId, node: c_int) -> c_double;
    fn roche_next_join(db: *mut c_void, a: RocheId, b: RocheId) -> c_double;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Abi,
    AbiVersionMismatch,
    InvalidId,
    NotFound,
    NulByte,
    Utf8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn last() -> Self {
        unsafe {
            let p = roche_last_error();
            if p.is_null() {
                return Self::new(ErrorKind::Abi, "RocheDB C ABI error");
            }
            let s = CStr::from_ptr(p).to_string_lossy();
            if s.is_empty() {
                Self::new(ErrorKind::Abi, "RocheDB C ABI error")
            } else {
                let message = s.into_owned();
                let kind = classify_abi_error(&message);
                Self::new(kind, message)
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl StdError for Error {}

impl From<NulError> for Error {
    fn from(value: NulError) -> Self {
        Self::new(ErrorKind::NulByte, value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: RocheId,
    pub score: f64,
    pub payload: Vec<u8>,
}

impl Hit {
    pub fn payload_utf8(&self) -> Result<&str, Error> {
        std::str::from_utf8(&self.payload).map_err(|e| Error::new(ErrorKind::Utf8, e.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedPayload {
    pub data: Vec<u8>,
    pub codec: PayloadCodec,
}

impl EncodedPayload {
    pub fn new(data: impl Into<Vec<u8>>, codec: PayloadCodec) -> Self {
        Self {
            data: data.into(),
            codec,
        }
    }

    pub fn raw(data: impl Into<Vec<u8>>) -> Self {
        Self::new(data, PayloadCodec::Raw)
    }

    pub fn json(data: impl Into<Vec<u8>>) -> Self {
        Self::new(data, PayloadCodec::Json)
    }

    pub fn nif(data: impl Into<Vec<u8>>) -> Self {
        Self::new(data, PayloadCodec::Nif)
    }

    pub fn bif(data: impl Into<Vec<u8>>) -> Self {
        Self::new(data, PayloadCodec::Bif)
    }

    pub fn as_utf8(&self) -> Result<&str, Error> {
        std::str::from_utf8(&self.data).map_err(|e| Error::new(ErrorKind::Utf8, e.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrieveStats {
    pub total_vectors: i32,
    pub scanned: i32,
    pub skipped_vectors: i32,
    pub returned: i32,
    pub rings_touched: i32,
    pub payload_bytes: i32,
    pub estimated_tokens: i32,
    pub fanout_nodes: i32,
    pub candidate_reduction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrieveResult {
    pub hits: Vec<Hit>,
    pub stats: RetrieveStats,
}

impl RetrieveResult {
    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn first(&self) -> Option<&Hit> {
        self.hits.first()
    }

    pub fn payloads(&self) -> impl Iterator<Item = &[u8]> {
        self.hits.iter().map(|hit| hit.payload.as_slice())
    }

    pub fn payload_strings(&self) -> Result<Vec<&str>, Error> {
        self.hits.iter().map(Hit::payload_utf8).collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectOptions {
    pub peers: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub auth_token: Option<String>,
    pub secret_key: Option<String>,
    pub galaxy: Option<String>,
}

impl ConnectOptions {
    pub fn new(peers: impl Into<String>) -> Self {
        Self {
            peers: peers.into(),
            ..Self::default()
        }
    }

    pub fn username(mut self, value: impl Into<String>) -> Self {
        self.username = Some(value.into());
        self
    }

    pub fn password(mut self, value: impl Into<String>) -> Self {
        self.password = Some(value.into());
        self
    }

    pub fn auth_token(mut self, value: impl Into<String>) -> Self {
        self.auth_token = Some(value.into());
        self
    }

    pub fn secret_key(mut self, value: impl Into<String>) -> Self {
        self.secret_key = Some(value.into());
        self
    }

    pub fn galaxy(mut self, value: impl Into<String>) -> Self {
        self.galaxy = Some(value.into());
        self
    }

    pub fn connect(self) -> Result<RocheDb, Error> {
        RocheDb::connect_options(&self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrieveOptions {
    pub ring: Option<String>,
    pub budget: i32,
    pub top_rings: i32,
    pub focus: i32,
}

impl Default for RetrieveOptions {
    fn default() -> Self {
        Self {
            ring: None,
            budget: 8,
            top_rings: 0,
            focus: 0,
        }
    }
}

impl RetrieveOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ring(mut self, value: impl Into<String>) -> Self {
        self.ring = Some(value.into());
        self
    }

    pub fn budget(mut self, value: i32) -> Self {
        self.budget = value;
        self
    }

    pub fn top_rings(mut self, value: i32) -> Self {
        self.top_rings = value;
        self
    }

    pub fn focus(mut self, value: i32) -> Self {
        self.focus = value;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadRingOptions {
    pub filter_json: Option<String>,
    pub selection: Option<String>,
    pub limit: i32,
    pub cursor: Option<String>,
    pub pagination: bool,
    pub page: i32,
    pub page_limit: i32,
    pub sort_field: Option<String>,
    pub sort_desc: bool,
}

impl Default for ReadRingOptions {
    fn default() -> Self {
        Self {
            filter_json: None,
            selection: None,
            limit: 100,
            cursor: None,
            pagination: false,
            page: 1,
            page_limit: 20,
            sort_field: None,
            sort_desc: true,
        }
    }
}

impl ReadRingOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter_json(mut self, value: impl Into<String>) -> Self {
        self.filter_json = Some(value.into());
        self
    }

    pub fn selection(mut self, value: impl Into<String>) -> Self {
        self.selection = Some(value.into());
        self
    }

    pub fn limit(mut self, value: i32) -> Self {
        self.limit = value;
        self
    }

    pub fn cursor(mut self, value: impl Into<String>) -> Self {
        self.cursor = Some(value.into());
        self
    }

    pub fn pagination(mut self, value: bool) -> Self {
        self.pagination = value;
        self
    }

    pub fn page(mut self, value: i32) -> Self {
        self.page = value;
        self
    }

    pub fn page_limit(mut self, value: i32) -> Self {
        self.page_limit = value;
        self
    }

    pub fn sort(mut self, field: impl Into<String>) -> Self {
        self.sort_field = Some(field.into());
        self.sort_desc = false;
        self
    }

    pub fn rsort(mut self, field: impl Into<String>) -> Self {
        self.sort_field = Some(field.into());
        self.sort_desc = true;
        self
    }
}

pub struct RocheDb {
    raw: *mut c_void,
}

unsafe impl Send for RocheDb {}

impl RocheDb {
    pub fn open_default() -> Result<Self, Error> {
        Self::open(8)
    }

    pub fn open(nodes: i32) -> Result<Self, Error> {
        init()?;
        let raw = unsafe { roche_open(nodes as c_int) };
        Self::from_raw(raw)
    }

    pub fn open_dir(nodes: i32, dir: &str) -> Result<Self, Error> {
        init()?;
        let dir = CString::new(dir)?;
        let raw = unsafe { roche_open_dir(nodes as c_int, dir.as_ptr()) };
        Self::from_raw(raw)
    }

    pub fn connect(peers: &str) -> Result<Self, Error> {
        init()?;
        let peers = CString::new(peers)?;
        let raw = unsafe { roche_connect(peers.as_ptr()) };
        Self::from_raw(raw)
    }

    pub fn connect_options(options: &ConnectOptions) -> Result<Self, Error> {
        Self::connect_auth(
            &options.peers,
            options.username.as_deref(),
            options.password.as_deref(),
            options.auth_token.as_deref(),
            options.secret_key.as_deref(),
            options.galaxy.as_deref(),
        )
    }

    pub fn connect_auth(
        peers: &str,
        username: Option<&str>,
        password: Option<&str>,
        auth_token: Option<&str>,
        secret_key: Option<&str>,
        galaxy: Option<&str>,
    ) -> Result<Self, Error> {
        init()?;
        let peers = CString::new(peers)?;
        let username = opt_cstring(username)?;
        let password = opt_cstring(password)?;
        let auth_token = opt_cstring(auth_token)?;
        let secret_key = opt_cstring(secret_key)?;
        let galaxy = opt_cstring(galaxy)?;
        let raw = unsafe {
            roche_connect_auth(
                peers.as_ptr(),
                opt_ptr(&username),
                opt_ptr(&password),
                opt_ptr(&auth_token),
                opt_ptr(&secret_key),
                opt_ptr(&galaxy),
            )
        };
        Self::from_raw(raw)
    }

    fn from_raw(raw: *mut c_void) -> Result<Self, Error> {
        if raw.is_null() {
            Err(Error::last())
        } else {
            Ok(Self { raw })
        }
    }

    pub fn now(&self) -> f64 {
        unsafe { roche_now(self.raw) }
    }

    pub fn advance(&self, dt: f64) {
        unsafe { roche_advance(self.raw, dt) };
    }

    pub fn configure_ring(&self, ring: &str, period: f64) -> Result<(), Error> {
        let ring = CString::new(ring)?;
        self.check(unsafe { roche_ring_configure(self.raw, ring.as_ptr(), period) })
    }

    pub fn set_galaxy_description(&self, description: &str) -> Result<(), Error> {
        let description = CString::new(description)?;
        self.check(unsafe { roche_set_galaxy_description(self.raw, description.as_ptr()) })
    }

    pub fn set_ring_description(&self, ring: &str, description: &str) -> Result<(), Error> {
        let ring = CString::new(ring)?;
        let description = CString::new(description)?;
        self.check(unsafe {
            roche_set_ring_description(self.raw, ring.as_ptr(), description.as_ptr())
        })
    }

    pub fn put(&self, ring: &str, payload: &[u8]) -> Result<RocheId, Error> {
        let ring = CString::new(ring)?;
        let mut id = empty_id();
        let data = if payload.is_empty() {
            ptr::null()
        } else {
            payload.as_ptr() as *const c_void
        };
        self.check(unsafe { roche_put(self.raw, ring.as_ptr(), data, payload.len(), &mut id) })?;
        Ok(id)
    }

    pub fn put_codec(
        &self,
        ring: &str,
        payload: &[u8],
        codec: PayloadCodec,
    ) -> Result<RocheId, Error> {
        let ring = CString::new(ring)?;
        let mut id = empty_id();
        let data = if payload.is_empty() {
            ptr::null()
        } else {
            payload.as_ptr() as *const c_void
        };
        self.check(unsafe {
            roche_put_codec(
                self.raw,
                ring.as_ptr(),
                data,
                payload.len(),
                codec.as_c(),
                &mut id,
            )
        })?;
        Ok(id)
    }

    pub fn put_str(&self, ring: &str, payload: &str) -> Result<RocheId, Error> {
        self.put(ring, payload.as_bytes())
    }

    pub fn put_json(&self, ring: &str, payload: &str) -> Result<RocheId, Error> {
        self.put_codec(ring, payload.as_bytes(), PayloadCodec::Json)
    }

    pub fn put_nif(&self, ring: &str, payload: &str) -> Result<RocheId, Error> {
        self.put_codec(ring, payload.as_bytes(), PayloadCodec::Nif)
    }

    pub fn put_bif(&self, ring: &str, payload: &[u8]) -> Result<RocheId, Error> {
        self.put_codec(ring, payload, PayloadCodec::Bif)
    }

    pub fn put_vec(&self, ring: &str, payload: &[u8], vec: &[f32]) -> Result<RocheId, Error> {
        let ring = CString::new(ring)?;
        let mut id = empty_id();
        let data = if payload.is_empty() {
            ptr::null()
        } else {
            payload.as_ptr() as *const c_void
        };
        let vec_ptr = if vec.is_empty() {
            ptr::null()
        } else {
            vec.as_ptr()
        };
        self.check(unsafe {
            roche_put_vec(
                self.raw,
                ring.as_ptr(),
                data,
                payload.len(),
                vec_ptr,
                vec.len(),
                &mut id,
            )
        })?;
        Ok(id)
    }

    pub fn put_vec_codec(
        &self,
        ring: &str,
        payload: &[u8],
        vec: &[f32],
        codec: PayloadCodec,
    ) -> Result<RocheId, Error> {
        let ring = CString::new(ring)?;
        let mut id = empty_id();
        let data = if payload.is_empty() {
            ptr::null()
        } else {
            payload.as_ptr() as *const c_void
        };
        let vec_ptr = if vec.is_empty() {
            ptr::null()
        } else {
            vec.as_ptr()
        };
        self.check(unsafe {
            roche_put_vec_codec(
                self.raw,
                ring.as_ptr(),
                data,
                payload.len(),
                codec.as_c(),
                vec_ptr,
                vec.len(),
                &mut id,
            )
        })?;
        Ok(id)
    }

    pub fn put_json_vec(&self, ring: &str, payload: &str, vec: &[f32]) -> Result<RocheId, Error> {
        self.put_vec_codec(ring, payload.as_bytes(), vec, PayloadCodec::Json)
    }

    pub fn put_nif_vec(&self, ring: &str, payload: &str, vec: &[f32]) -> Result<RocheId, Error> {
        self.put_vec_codec(ring, payload.as_bytes(), vec, PayloadCodec::Nif)
    }

    pub fn put_bif_vec(&self, ring: &str, payload: &[u8], vec: &[f32]) -> Result<RocheId, Error> {
        self.put_vec_codec(ring, payload, vec, PayloadCodec::Bif)
    }

    pub fn get(&self, id: RocheId) -> Result<Option<Vec<u8>>, Error> {
        let mut len = 0usize;
        let p = unsafe { roche_get(self.raw, id, &mut len) };
        if p.is_null() {
            let err = Error::last();
            if err.message.contains("not found") || err.message.contains("key not found") {
                return Ok(None);
            }
            return Err(err);
        }
        Ok(Some(unsafe { take_buffer(p, len) }))
    }

    pub fn get_encoded(&self, id: RocheId) -> Result<Option<EncodedPayload>, Error> {
        let mut len = 0usize;
        let mut codec = ROCHE_CODEC_RAW;
        let p = unsafe { roche_get_codec(self.raw, id, &mut len, &mut codec) };
        if p.is_null() {
            let err = Error::last();
            if err.message.contains("not found") || err.message.contains("key not found") {
                return Ok(None);
            }
            return Err(err);
        }
        Ok(Some(EncodedPayload {
            data: unsafe { take_buffer(p, len) },
            codec: PayloadCodec::from_c(codec)?,
        }))
    }

    pub fn get_string(&self, id: RocheId) -> Result<Option<String>, Error> {
        self.get(id)?
            .map(String::from_utf8)
            .transpose()
            .map_err(|e| Error::new(ErrorKind::Utf8, e.to_string()))
    }

    pub fn batch_get(&self, ids: &[RocheId]) -> Result<Vec<Vec<u8>>, Error> {
        let ptr = if ids.is_empty() {
            ptr::null()
        } else {
            ids.as_ptr()
        };
        let r = unsafe { roche_batch_get(self.raw, ptr, ids.len()) };
        if r.is_null() {
            return Err(Error::last());
        }
        let result = unsafe {
            let values = slice::from_raw_parts((*r).values, (*r).len);
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                let bytes = if value.data.is_null() {
                    Vec::new()
                } else {
                    slice::from_raw_parts(value.data as *const u8, value.len).to_vec()
                };
                out.push(bytes);
            }
            roche_batch_get_free(r);
            out
        };
        Ok(result)
    }

    pub fn query(&self, id: RocheId, selection: &str) -> Result<Vec<u8>, Error> {
        let selection = CString::new(selection)?;
        let mut len = 0usize;
        let p = unsafe { roche_query(self.raw, id, selection.as_ptr(), &mut len) };
        if p.is_null() {
            return Err(Error::last());
        }
        Ok(unsafe { take_buffer(p, len) })
    }

    pub fn query_string(&self, id: RocheId, selection: &str) -> Result<String, Error> {
        String::from_utf8(self.query(id, selection)?)
            .map_err(|e| Error::new(ErrorKind::Utf8, e.to_string()))
    }

    pub fn read_ring_json(&self, ring: &str, options: &ReadRingOptions) -> Result<String, Error> {
        let ring = CString::new(ring)?;
        let filter_json = opt_cstring(options.filter_json.as_deref())?;
        let selection = opt_cstring(options.selection.as_deref())?;
        let cursor = opt_cstring(options.cursor.as_deref())?;
        let sort_field = opt_cstring(options.sort_field.as_deref())?;
        let mut len = 0usize;
        let p = unsafe {
            roche_read_ring_json(
                self.raw,
                ring.as_ptr(),
                opt_ptr(&filter_json),
                opt_ptr(&selection),
                options.limit as c_int,
                opt_ptr(&cursor),
                if options.pagination { 1 } else { 0 },
                options.page as c_int,
                options.page_limit as c_int,
                opt_ptr(&sort_field),
                if options.sort_desc { 1 } else { 0 },
                &mut len,
            )
        };
        if p.is_null() {
            return Err(Error::last());
        }
        String::from_utf8(unsafe { take_buffer(p, len) })
            .map_err(|e| Error::new(ErrorKind::Utf8, e.to_string()))
    }

    pub fn retrieve(
        &self,
        vec: &[f32],
        ring: Option<&str>,
        budget: i32,
        top_rings: i32,
        focus: i32,
    ) -> Result<RetrieveResult, Error> {
        let ring = opt_cstring(ring)?;
        let vec_ptr = if vec.is_empty() {
            ptr::null()
        } else {
            vec.as_ptr()
        };
        let r = unsafe {
            roche_retrieve(
                self.raw,
                vec_ptr,
                vec.len(),
                opt_ptr(&ring),
                budget,
                top_rings,
                focus,
            )
        };
        if r.is_null() {
            return Err(Error::last());
        }
        let result = unsafe {
            let raw = &*r;
            let raw_hits = slice::from_raw_parts(raw.hits, raw.len);
            let hits = raw_hits
                .iter()
                .map(|h| Hit {
                    id: h.id,
                    score: h.score,
                    payload: if h.payload.is_null() {
                        Vec::new()
                    } else {
                        slice::from_raw_parts(h.payload as *const u8, h.payload_len).to_vec()
                    },
                })
                .collect();
            let stats = RetrieveStats {
                total_vectors: raw.total_vectors,
                scanned: raw.scanned,
                skipped_vectors: raw.skipped_vectors,
                returned: raw.returned,
                rings_touched: raw.rings_touched,
                payload_bytes: raw.payload_bytes,
                estimated_tokens: raw.estimated_tokens,
                fanout_nodes: raw.fanout_nodes,
                candidate_reduction: raw.candidate_reduction,
            };
            roche_retrieve_free(r);
            RetrieveResult { hits, stats }
        };
        Ok(result)
    }

    pub fn retrieve_with(
        &self,
        vec: &[f32],
        options: RetrieveOptions,
    ) -> Result<RetrieveResult, Error> {
        self.retrieve(
            vec,
            options.ring.as_deref(),
            options.budget,
            options.top_rings,
            options.focus,
        )
    }

    pub fn atlas(
        &self,
        query_vec: Option<&[f32]>,
        max_centroid_dims: i32,
    ) -> Result<String, Error> {
        let vec = query_vec.unwrap_or(&[]);
        let vec_ptr = if vec.is_empty() {
            ptr::null()
        } else {
            vec.as_ptr()
        };
        let mut len = 0usize;
        let p = unsafe { roche_atlas(self.raw, vec_ptr, vec.len(), max_centroid_dims, &mut len) };
        if p.is_null() {
            return Err(Error::last());
        }
        let bytes = unsafe { take_buffer(p, len) };
        String::from_utf8(bytes).map_err(|e| Error::new(ErrorKind::Utf8, e.to_string()))
    }

    pub fn locate(&self, id: RocheId, at: Option<f64>) -> Result<i32, Error> {
        let node = unsafe { roche_locate(self.raw, id, at.unwrap_or(-1.0)) };
        if node < 0 {
            Err(Error::last())
        } else {
            Ok(node)
        }
    }

    pub fn next_visit(&self, id: RocheId, node: i32) -> Result<f64, Error> {
        let t = unsafe { roche_next_visit(self.raw, id, node) };
        if t < 0.0 {
            Err(Error::last())
        } else {
            Ok(t)
        }
    }

    pub fn next_join(&self, a: RocheId, b: RocheId) -> Result<Option<f64>, Error> {
        let t = unsafe { roche_next_join(self.raw, a, b) };
        if t < 0.0 {
            Ok(None)
        } else {
            Ok(Some(t))
        }
    }

    fn check(&self, rc: c_int) -> Result<(), Error> {
        if rc == ROCHE_OK {
            Ok(())
        } else {
            Err(Error::last())
        }
    }
}

impl Drop for RocheDb {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { roche_close(self.raw) };
            self.raw = ptr::null_mut();
        }
    }
}

fn init() -> Result<(), Error> {
    INIT.call_once(|| unsafe { roche_init() });
    let version = unsafe { roche_abi_version() };
    if version != ROCHE_ABI_VERSION {
        Err(Error::new(
            ErrorKind::AbiVersionMismatch,
            format!(
                "RocheDB ABI version mismatch: expected {}, got {}",
                ROCHE_ABI_VERSION, version
            ),
        ))
    } else {
        Ok(())
    }
}

fn classify_abi_error(message: &str) -> ErrorKind {
    let lower = message.to_ascii_lowercase();
    if lower.contains("not found") || lower.contains("key not found") {
        ErrorKind::NotFound
    } else {
        ErrorKind::Abi
    }
}

fn parse_id_part<T>(parts: &mut std::str::Split<'_, char>, name: &str) -> Result<T, Error>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    let raw = parts.next().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidId,
            format!("invalid RocheDB id: missing {}", name),
        )
    })?;
    raw.parse::<T>().map_err(|e| {
        Error::new(
            ErrorKind::InvalidId,
            format!("invalid RocheDB id field '{}': {}", name, e),
        )
    })
}

fn empty_id() -> RocheId {
    RocheId {
        parent: 0,
        epoch: 0,
        seq: 0,
        t_write: 0.0,
    }
}

fn opt_cstring(value: Option<&str>) -> Result<Option<CString>, Error> {
    value.map(CString::new).transpose().map_err(Error::from)
}

fn opt_ptr(value: &Option<CString>) -> *const c_char {
    value.as_ref().map_or(ptr::null(), |s| s.as_ptr())
}

unsafe fn take_buffer(p: *mut c_void, len: usize) -> Vec<u8> {
    let bytes = slice::from_raw_parts(p as *const u8, len).to_vec();
    roche_free(p);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_roundtrip_retrieve_and_atlas() {
        let db = RocheDb::open_default().unwrap();
        db.set_galaxy_description("Rust test galaxy").unwrap();
        db.set_ring_description("docs/rust", "Rust driver documents")
            .unwrap();
        db.configure_ring("docs/rust", 30.0).unwrap();

        let payload = br#"{"title":"hello rust","lang":"rust"}"#;
        let id = db
            .put_json_vec(
                "docs/rust",
                std::str::from_utf8(payload).unwrap(),
                &[1.0, 0.0],
            )
            .unwrap();
        assert!(!id.is_empty());
        assert!(id.to_string().contains(':'));
        assert_eq!(RocheId::parse(&id.to_string()).unwrap(), id);
        assert_eq!(db.get(id).unwrap().unwrap(), payload);
        let encoded = db.get_encoded(id).unwrap().unwrap();
        assert_eq!(encoded.codec, PayloadCodec::Json);
        assert_eq!(encoded.data, payload);
        assert_eq!(
            encoded.as_utf8().unwrap(),
            r#"{"title":"hello rust","lang":"rust"}"#
        );
        assert_eq!(
            db.get_string(id).unwrap().unwrap(),
            r#"{"title":"hello rust","lang":"rust"}"#
        );

        let values = db.batch_get(&[id]).unwrap();
        assert_eq!(values, vec![payload.to_vec()]);

        let selected = db.query_string(id, "{ title }").unwrap();
        assert!(selected.contains("hello rust"));

        let page = db
            .read_ring_json(
                "docs/rust",
                &ReadRingOptions::new()
                    .filter_json(r#"{"lang":"rust"}"#)
                    .selection("{ title }")
                    .limit(1)
                    .rsort("time"),
            )
            .unwrap();
        assert!(page.contains(r#""items""#));
        assert!(page.contains(r#""count":1"#));
        assert!(page.contains("hello rust"));
        assert!(page.contains(r#""sort":"time""#));
        assert!(page.contains(r#""sortDirection":"desc""#));

        let bif_id = db
            .put_bif_vec("artifacts/bif", &[1, 2, 3, 4], &[0.0, 1.0])
            .unwrap();
        let bif_encoded = db.get_encoded(bif_id).unwrap().unwrap();
        assert_eq!(bif_encoded.codec, PayloadCodec::Bif);
        assert_eq!(bif_encoded.data, vec![1, 2, 3, 4]);
        let bif_page = db
            .read_ring_json("artifacts/bif", &ReadRingOptions::new().limit(1))
            .unwrap();
        assert!(bif_page.contains(r#""codec":"bif""#));
        assert!(bif_page.contains(r#""encoding":"base64""#));

        let rr = db
            .retrieve_with(
                &[1.0, 0.0],
                RetrieveOptions::new().ring("docs/rust").budget(4),
            )
            .unwrap();
        assert_eq!(rr.hits.len(), 1);
        assert!(!rr.is_empty());
        assert!(rr.first().is_some());
        assert_eq!(rr.stats.scanned, 1);
        assert_eq!(rr.payloads().collect::<Vec<_>>(), vec![payload.as_slice()]);
        assert_eq!(
            rr.payload_strings().unwrap(),
            vec![r#"{"title":"hello rust","lang":"rust"}"#]
        );
        assert_eq!(
            rr.hits[0].payload_utf8().unwrap(),
            r#"{"title":"hello rust","lang":"rust"}"#
        );

        let atlas = db.atlas(Some(&[1.0, 0.0]), 8).unwrap();
        assert!(atlas.contains("Rust test galaxy"));
        assert!(atlas.contains("Rust driver documents"));

        let node = db.locate(id, None).unwrap();
        assert!(node >= 0);
        assert!(db.next_visit(id, node).unwrap() >= 0.0);

        let now = db.now();
        db.advance(2.5);
        assert!(db.now() >= now + 2.5);
    }

    #[test]
    fn errors_include_c_abi_message() {
        let db = RocheDb::open(8).unwrap();
        let err = db.put("bad\0ring", b"x").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NulByte);
        assert!(err.to_string().contains("nul byte"));
    }

    #[test]
    fn id_parse_reports_invalid_input() {
        let parsed = RocheId::parse("1:2:3:4.5").unwrap();
        assert_eq!(parsed, RocheId::new(1, 2, 3, 4.5));
        assert_eq!(parsed.to_string(), "1:2:3:4.5");
        assert_eq!("1:2:3:4.5".parse::<RocheId>().unwrap(), parsed);

        let err = RocheId::parse("1:2:3").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidId);
        assert!(err.message().contains("missing"));

        let err = RocheId::parse("1:2:3:4:5").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidId);
        assert!(err.message().contains("too many"));
    }

    #[test]
    fn open_dir_reopens_persisted_data() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "rochedb-rust-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let id = {
            let db = RocheDb::open_dir(4, dir.to_str().unwrap()).unwrap();
            db.put_str("persist/rust", "persistent rust payload")
                .unwrap()
        };

        let db = RocheDb::open_dir(4, dir.to_str().unwrap()).unwrap();
        assert_eq!(
            db.get_string(id).unwrap().unwrap(),
            "persistent rust payload"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
