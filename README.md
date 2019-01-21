# tantivy-node

node bindings for [tantivy](https://github.com/tantivy-search/tantivy), using [neon](https://github.com/neon-bindings/neon)

*Work in progress*

## Usage

```javascript
const tantivy = require('tantivy-node')

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

const docs = [
  { title: 'Hello, world', body: 'and it rusts down here', url: 'a:1' },
  { title: 'Sesame', body: 'the future is brightly broken', url: 'a:2' }
]

const path = '/tmp/tantivy-test2'

const index = tantivy()
index.createInDir(path, schema)

// on subsequent runs:
// index.openInDir(path)

docs.forEach(doc => index.addDocument(doc))

const res = index.query('future')
console.log(res)

