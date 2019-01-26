extern crate napi_rs;
extern crate tantivy;

use napi_rs::*;
use napi_rs::Value;

use tantivy::Index;
use tantivy::IndexWriter;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema;
use tantivy::schema::{Value as SValue, Document, NamedFieldDocument};

use std::path::PathBuf;
use std::fmt;

struct IndexHandle {
    pub index: Index,
    pub writer: Option<IndexWriter>
}

impl IndexHandle {
    fn assert_writer(&mut self) -> Result<(&mut Index, &mut IndexWriter)> {
        let writer = self.writer.as_mut();
        match writer {
            Some(writer) => Ok((&mut self.index, writer)),
            None => {
                eprintln!("Tantivy: Cannot write, no index writer open.");
                Err(napi_rs::Error::new(Status::GenericFailure))
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
    ctx.export("open_in_dir", callback!(open_in_dir));
    ctx.export("create_in_dir", callback!(create_in_dir));

    ctx.export("index_query", callback!(index_query));
    ctx.export("index_writer", callback!(index_writer));

    ctx.export("index_writer_add_document", callback!(index_writer_add_document));
    ctx.export("index_writer_commit", callback!(index_writer_commit));

    Ok(None)
}

fn open_in_dir(ctx: CallContext) -> AnyResult {
    let path = ctx.args[0].to_string();
    let index_path = PathBuf::from(path);

    let index = Index::open_in_dir(index_path).unwrap();

    let handle = IndexHandle { index, writer: None };

    let mut wrapper = ctx.env.create_object();
    ctx.env.wrap(&mut wrapper, handle)?;
    wrapper.into_result()
}

fn create_in_dir(ctx: CallContext) -> AnyResult {
    let path = ctx.args[0].to_string();
    let path = PathBuf::from(path);
    let schema = ctx.args[1].to_string();
    let schema: schema::Schema = serde_json::from_str(&schema).unwrap();

    let index = Index::create_in_dir(path, schema).unwrap();
    let handle = IndexHandle { index, writer: None };

    let mut wrapper = ctx.env.create_object();
    ctx.env.wrap(&mut wrapper, handle)?;
    wrapper.into_result()
}

fn index_writer(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);

    let writer = {
        let index: &mut Index = &mut handle.index;
        let writer = index.writer(50_000_000).unwrap();
        writer
    };

    handle.writer = Some(writer);

    ctx.env.get_undefined().into_result()
}

fn index_writer_add_document(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);
    let (index, writer) = handle.assert_writer()?;

    let doc: Value<Object> = ctx.args[1].try_into().unwrap();

    // let is_array: bool = doc.is_array()?;
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
    // println!("document: {:?}", document);

    let opstamp = writer.add_document(document);
    ctx.env.create_int64(opstamp as i64).into_result()
}

fn index_writer_commit(ctx: CallContext) -> AnyResult {
    let handle: &mut IndexHandle = ctx.args[0].unwrap(ctx.env);
    let (_index, writer) = handle.assert_writer()?;
    let opstamp = writer.commit().unwrap();
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

    let array: Value<Object> = docs.to_js(&ctx)?;

    array.into_result()
}

// Typing helpers.

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
