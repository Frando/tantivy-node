extern crate napi_rs;
extern crate tantivy;

// use napi_rs::{Result, Error, Any, Value, Object, AnyResult, ValueType, Ref, Function, CallContext, Status, Env, ModuleInitContext};
use napi_rs::{Any, Value, Object, ValueType, Ref, Function, CallContext, Status, Env, ModuleInitContext};
use napi_rs::{callback, register_module};

use tantivy::{Index, IndexWriter, TantivyError};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema;
use tantivy::schema::{Value as SValue, Document, NamedFieldDocument};

use std::path::PathBuf;
use std::fmt;

pub type AnyResult = Result<Option<Value<'static, Any>>>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    status: Status,
    msg: String
}

impl Error {
    fn new (status: Status) -> Self {
        Error { status: status, msg: "foo".to_string() }
    }
}

impl From<tantivy::TantivyError> for Error {
    fn from(e: TantivyError) -> Self {
        eprintln!("Tantivy error: {}", e);
        Error::new(Status::GenericFailure)
    }
}
impl From<napi_rs::Error> for Error {
    fn from(e: napi_rs::Error) -> Self {
        eprintln!("Napi error: {:?}", e);
        Error::new(e.get_status())
    }
}

// impl From<TantivyNodeError> for Error {
    // fn from(e: TantivyNodeError) -> Self {
        // Error::new(Status::GenericFailure)
    // }
// }

// impl From<tantivy::TantivyError> for napi_rs::Error {
    // fn from(e: tantivy::TantivyError) -> Self {
        // Error::new(Status::GenericFailure)
    // }
// }

struct IndexHandle {
    pub index: Index,
    pub writer: Option<IndexWriter>
}

// Somehow, the IndexHandle never gets dropped.
// This seems to be a bug/missing feature in napi-rs, though
// impl Drop for IndexHandle {
    // fn drop (&mut self) {
        // println!("IndexHandle is dropped!")
    // }
// }

impl IndexHandle {
    fn assert_writer(&mut self) -> Result<(&mut Index, &mut IndexWriter)> {
        let writer = self.writer.as_mut();
        match writer {
            Some(writer) => Ok((&mut self.index, writer)),
            None => {
                eprintln!("Tantivy: Cannot write, no index writer open.");
                Err(Error::new(Status::GenericFailure))
            }
        }
    }
}

impl fmt::Debug for IndexHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let writer = match &self.writer {
            Some(_x) => "Yes",
            None => "No"
        };
        write!(f, "IndexHandle {{ index: {:?}, writer: {:} }}", &self.index, writer)
    }
}

register_module!(tantivy, init);

fn init<'env>(mut ctx: ModuleInitContext) -> Result<Option<Value<'env, Object>>> {
    ctx.export("index_open_in_dir", callback!(index_open_in_dir));
    ctx.export("index_create_in_dir", callback!(index_create_in_dir));
    ctx.export("index_create_in_ram", callback!(index_create_in_ram));

    ctx.export("index_query", callback!(index_query));

    ctx.export("index_writer_open", callback!(index_writer_open));
    ctx.export("index_writer_add_document", callback!(index_writer_add_document));
    ctx.export("index_writer_commit", callback!(index_writer_commit));

    // ctx.export("foo", callback!(foo));

    Ok(None)
}

fn index_open_in_dir(ctx: CallContext) -> AnyResult {
    let path = ctx.args[0].to_string();
    let index_path = PathBuf::from(path);

    let index = Index::open_in_dir(index_path).unwrap();

    let handle = IndexHandle { index, writer: None };

    wrap_into_result(&ctx, handle)
}


fn index_create_in_dir(ctx: CallContext) -> AnyResult {
    let path = ctx.args[0].to_string();
    let path = PathBuf::from(path);
    let schema = ctx.args[1].to_string();
    let schema: schema::Schema = serde_json::from_str(&schema)
        .map_err(|e| js_error(e, Status::ObjectExpected))?;

    let index = Index::create_in_dir(path, schema).unwrap();
    let handle = IndexHandle { index, writer: None };
    wrap_into_result(&ctx, handle)
}

fn index_create_in_ram(ctx: CallContext) -> AnyResult {
    let schema = ctx.args[0].to_string();
    let schema: schema::Schema = serde_json::from_str(&schema)
        .map_err(|e| js_error(e, Status::ObjectExpected))?;

    let index = Index::create_in_ram(schema);
    let handle = IndexHandle { index, writer: None };
    wrap_into_result(&ctx, handle)
}

fn index_writer_open(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);

    // If a writer exists this is a noop.
    if let Some(_) = handle.writer {
        return undefined(ctx)
    }

    let writer = {
        let index: &mut Index = &mut handle.index;
        let writer = index.writer(50_000_000)?;
            // .map_err(|e| js_error(e, Status::GenericFailure))?;
        writer
    };

    handle.writer = Some(writer);

    return undefined(ctx)
}

fn index_writer_add_document(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);
    let (index, writer) = handle.assert_writer()?;

    let doc: Value<Object> = ctx.args[1].try_into().unwrap();

    let len = doc.get_array_length()?;

    let schema = index.schema();

    let mut document = Document::default();

    for i in 0..len {
        let obj: Value<Object> = doc.get_index(i)?;
        let fieldname: std::string::String = obj.get_named_property("field").unwrap().to_string();
        let value: std::string::String = obj.get_named_property("value").unwrap().to_string();
        let field = schema.get_field(&fieldname).unwrap();
        document.add_text(field, &value)
    }

    let opstamp = writer.add_document(document);
    writer.commit().unwrap();
    ctx.env.create_int64(opstamp as i64).into_result()
}

fn index_writer_commit(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);
    let (index, writer) = handle.assert_writer()?;
    let opstamp = writer.commit().unwrap();
    index.load_searchers().unwrap();
    ctx.env.create_int64(opstamp as i64).into_result()
}

fn index_query(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);
    let query = ctx.args[1].to_string();
    let mut limit = 10;
    if ctx.args.len() > 2 {
        limit = ctx.args[2].i64();
    }

    let index: &mut Index = &mut handle.index;

    let docs = tantivy_query(index, query, limit as usize);

    let array: Value<Object> = docs.to_js(&ctx)?;
    array.into_result()
}

fn tantivy_query (index: &mut Index, query: String, limit: usize) -> Vec<NamedFieldDocument> {
    let searcher = index.searcher();
    let schema = index.schema();

    let mut fields = vec![];
    for field in schema.fields() {
        if field.is_indexed() {
            fields.push(schema.get_field(field.name()).unwrap())
        }
    }

    let query_parser = QueryParser::for_index(&index, fields);
    let query = query_parser.parse_query(&query).unwrap();

    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit as usize)).unwrap();
    // println!("top_docs {:?}", top_docs);

    let mut docs = Vec::new();
    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address).unwrap();
        let named_doc = schema.to_named_doc(&retrieved_doc);
        docs.push(named_doc);
    }
    docs
}

// Typing helpers.

fn js_error<T: fmt::Display>(e: T, status: Status) -> Error {
    eprintln!("Tantivy error: {}", e);
    Error::new(status)
}

// fn map_err<T: fmt::Display>(e: T) -> Error {
    // eprintln!("Tantivy error: {}", e);
    // Error::new(Status::GenericFailure)
// }

fn wrap_into_result<'a, T: 'static>(ctx: &'a CallContext, handle: T) -> AnyResult {
    let mut wrapper = ctx.env.create_object();
    ctx.env.wrap(&mut wrapper, handle).unwrap();
    wrapper.into_result()
}

fn undefined(ctx: CallContext) -> AnyResult {
    ctx.env.get_undefined().into_result()
}

trait ConvertToJs<'a, T> {
    fn to_js (&self, ctx: &'a CallContext) -> Result<Value<'a, T>>;
}

impl <'a, T> ConvertToJs<'a, Object> for std::vec::Vec<T> 
    where T: ConvertToJs<'a, Object> {
    fn to_js(&self, ctx: &'a CallContext) -> Result<Value<'a, Object>> {
        let mut arr = ctx.env.create_array_with_length(self.len());
        for (i, val) in self.iter().enumerate() {
            let val = val.to_js(&ctx)?;
            arr.set_index(i, val)?;
        }
        Ok(arr)
    }
}


impl<'a> ConvertToJs<'a, Object> for NamedFieldDocument {
    fn to_js(&self, ctx: &'a CallContext) -> Result<Value<'a, Object>> {
        let mut obj = ctx.env.create_object();
        for (key, value) in self.0.iter() {
            let mut arr = ctx.env.create_array_with_length(value.len());
            for (i, val) in value.iter().enumerate() {
                match val {
                    SValue::Str(ref v) => {
                        arr.set_index(i, ctx.env.create_string(v))?;
                    },
                    SValue::U64(u) => {
                        arr.set_index(i, ctx.env.create_int64(*u as i64))?;
                    },
                    SValue::I64(u) => {
                        arr.set_index(i, ctx.env.create_int64(*u))?;
                    },
                    SValue::Facet(ref facet) => {
                        arr.set_index(i, ctx.env.create_string(&facet.to_string()))?;
                    },
                    SValue::Bytes(ref _bytes) => {
                        panic!("byte fields are not supported");
                    }
                };
            }
            obj.set_named_property(key, arr)?;
        }
        Ok(obj)
    }
}

// the following is mostly taken from
// https://github.com/cztomsik/node-webrender/blob/master/native/src/lib.rs

trait IntoAnyResult {
    fn into_result(&self) -> AnyResult;
}

impl<'env, T: ValueType> IntoAnyResult for Value<'env, T> {
    fn into_result(&self) -> AnyResult {
        unsafe {
            let any: Value<'env, Any> = self.try_into().unwrap();
            Ok(Some(std::mem::transmute(any)))
        }
    }
}

impl IntoAnyResult for () {
    fn into_result(&self) -> AnyResult {
        Ok(None)
    }
}

trait ConvertToRs<'env> {
    fn to_string(&self) -> std::string::String;
    fn f64(&self) -> f64;
    fn f32(&self) -> f32;
    fn i64(&self) -> i64;
    fn i32(&self) -> i32;
    fn unwrap<T: 'static>(&self, env: &'env Env) -> &'env mut T;
    fn unwrap_opt<T: 'static>(&self, env: &'env Env) -> Option<&'env mut T>;
    fn cb(&self, env: &'env Env) -> Ref<Function>;
}

impl<'env> ConvertToRs<'env> for Value<'env, Any> {
    fn to_string(&self) -> std::string::String {
        let codepoints: Vec<u16> = self.coerce_to_string().unwrap().into();
        std::string::String::from_utf16(&codepoints[..]).unwrap()
    }

    fn f64(&self) -> f64 {
        self.coerce_to_number().unwrap().into()
    }

    fn f32(&self) -> f32 {
        self.f64() as f32
    }

    fn i64(&self) -> i64 {
        self.coerce_to_number().unwrap().into()
    }

    fn i32(&self) -> i32 {
        self.i64() as i32
    }

    fn unwrap<T: 'static>(&self, env: &'env Env) -> &'env mut T {
        self.unwrap_opt(env).unwrap()
    }

    fn unwrap_opt<T: 'static>(&self, env: &'env Env) -> Option<&'env mut T> {
        let js_object = self.try_into().ok();
        js_object.map(|o| env.unwrap(&o).unwrap())
    }

    fn cb(&self, env: &'env Env) -> Ref<Function> {
        let f: Value<Function> = self.try_into().unwrap_or_else(|err| unsafe {
            panic!(
                "expected cb, found {:?}, err {:?}",
                self.get_value_type(),
                err
            )
        });

        env.create_reference(&f)
    }
}

// struct IndexWriterHandle {
    // pub writer: IndexWriter,
// }

// impl <'a> ConvertToJs<'a, Object> for IndexHandle {
    // fn to_js(&self, ctx: &'a CallContext) -> Result<Value<'a, Object>> {
        // let res = wrap_into_result(&ctx, *self);
        // Ok(res)
    // }
// }

// fn wrap_object<'a, T>(ctx: &'a CallContext, object: &T) -> Result<Value<'a, Object>> {
    // let mut wrapper = ctx.env.create_object();
    // ctx.env.wrap(&mut wrapper, *object)?;
    // Ok(wrapper)
// }
