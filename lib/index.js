const tantivy = require('../native/index.node')

console.log('lib', tantivy)

const schema = [
  {
    name: 'title',
    type: 'text',
    options: {
      indexing: {
        record: 'position',
        tokenizer: 'en_stem'
      },
      stored: true
    }
  },
  {
    name: 'body',
    type: 'text',
    options: {
      indexing: {
        record: 'position',
        tokenizer: 'en_stem'
      },
      stored: true
    }
  },
  {
    name: 'url',
    type: 'text',
    options: {
      indexing: null,
      stored: true
    }
  }
]
let path = '/tmp/tantivy-test'
let res
// const handle  = tantivy.create_in_dir(path, JSON.stringify(schema))
const handle  = tantivy.open_in_dir(path)
console.log('js index', handle)

res = tantivy.index_writer(handle)
console.log('js index writer', res)

let doc = [
  { field: 'body', value: 'hello from node' },
  { field: 'title', value: 'hi there' }
]
console.log('js doc', doc)
res = tantivy.index_writer_add_document(handle, doc)
console.log('js add document', res)

res = tantivy.index_writer_commit(handle)
console.log('commit res', res)

// res = tantivy.index_query(handle, 'future')
// console.log('res for "future"', res)

res = tantivy.index_query(handle, 'node', 20)
console.log('res for "node"', res)
