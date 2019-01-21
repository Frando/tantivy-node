#[macro_use]
extern crate neon;
#[macro_use]
extern crate neon_serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate tantivy;

use neon::prelude::*;

use tantivy::Index;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;

use neon_serde::to_value;


use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct JsFieldValue {
    pub field: String,
    pub value: String
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct JsDoc {
    field_values: Vec<JsFieldValue>
}

pub struct IndexHandle {
    index: Option<Index>
}

impl IndexHandle {
    fn setIndex (&mut self, index: Index) {
        self.index = Some(index)
    }
}

declare_types! {
    pub class JsIndex for IndexHandle {
        init (mut cx) {
            let handle = IndexHandle {
                index: None
            };
            Ok(handle)
        }

        method createInDir(mut cx) {
            let path_js = cx.argument::<JsString>(0)?.value();
            let schema_json = cx.argument::<JsString>(1)?.value();

            let index_path = PathBuf::from(path_js);
            let schema: Schema = serde_json::from_str(&schema_json).unwrap();

            let index = Index::create_in_dir(index_path, schema)
                    .or_else(|err| cx.throw_error(err.to_string()))?;

            {
                let mut this = cx.this();
                let guard = cx.lock();
                cx.borrow_mut(&mut this, |mut handle| handle.setIndex(index))
            }

            Ok(cx.boolean(true).upcast())
        }

        method openInDir(mut cx) {
            let path_js = cx.argument::<JsString>(0)?.value();
            let index_path = PathBuf::from(path_js);

            let index = Index::open_in_dir(index_path)
                    .or_else(|err| cx.throw_error(err.to_string()))?;

            {
                let mut this = cx.this();
                let guard = cx.lock();
                cx.borrow_mut(&mut this, |mut handle| handle.setIndex(index))
            }

            Ok(cx.boolean(true).upcast())
        }


        method commit(mut cx) {
            let this = cx.this();
            let result = cx.borrow(&this, |handle| {
                let index = (&handle.index).as_ref().unwrap();
                let mut index_writer = index.writer(50_000_000).unwrap();
                let res = index_writer.commit();
                // println!("commit res {:?}", res);
                let searcher = index.searcher();
                // println!("commit num docs {:?}", searcher.num_docs());
                res
            });
            result.or_else(|err| cx.throw_error(err.to_string()))?;
            Ok(cx.undefined().upcast())
        }

        method addDocument(mut cx) {
            let doc_js = cx.argument::<JsValue>(0)?;
            let doc_parsed: JsDoc = neon_serde::from_value(&mut cx, doc_js)?;

            let this = cx.this();
            cx.borrow(&this, |handle| {
                let index = (&handle.index).as_ref().unwrap();

                let mut index_writer = index.writer(50_000_000).unwrap();
                let schema = index.schema();

                let mut document = Document::default();

                for field_value in doc_parsed.field_values {
                    let field = schema.get_field(&field_value.field).unwrap();
                    document.add_text(field, &field_value.value)
                }

                // println!("document {:?}", document);
                index_writer.add_document(document);
                index_writer.commit();
            });
            Ok(cx.undefined().upcast())
        }

        method query(mut cx) {
            let query = cx.argument::<JsString>(0)?.value();
            // println!("query {:?}", query);

            let this = cx.this();
            let docs = {
                let guard = cx.lock();
                let handle = this.borrow(&guard);
                let index = (&handle.index).as_ref().unwrap();
                let searcher = index.searcher();

                let schema = index.schema();

                // todo: query parsing and schema handling has to be dynamic.
                let title = schema.get_field("title").unwrap();
                let body = schema.get_field("body").unwrap();

                let query_parser = QueryParser::for_index(&index, vec![title, body]);
                let query = query_parser.parse_query(&query).unwrap();

                // println!("num docs {:?}", searcher.num_docs());

                // todo: make limit configurable.
                let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
                // println!("top_docs {:?}", top_docs);

                let mut docs = Vec::new();
                for (_score, doc_address) in top_docs {
                    let retrieved_doc = searcher.doc(doc_address).unwrap();
                    let named_doc = schema.to_named_doc(&retrieved_doc);
                    docs.push(named_doc);
                }
                docs
            };

            // Create the JS array
            let js_array: Handle<JsArray> = JsArray::new(&mut cx, docs.len() as u32);

            // Iterate over the rust Vec and map each value in the Vec to the JS array
            docs.iter().enumerate().for_each(|e| {
                let (i, named_doc) = e;
                let js_value = neon_serde::to_value(&mut cx, named_doc).unwrap();
                js_array.set(&mut cx, i as u32, js_value);
            });

            Ok(js_array.upcast())
        }
    }
}

register_module!(mut m, {
    m.export_class::<JsIndex>("Index")?;
    Ok(())
});

