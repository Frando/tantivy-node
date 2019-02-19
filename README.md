# tantivy-node

node bindings for [tantivy](https://github.com/tantivy-search/tantivy), using [neon](https://github.com/neon-bindings/neon)

*Work in progress*

## Usage

```javascript
const tantivy = require('tantivy-node')

// Define a schema. See tantivy docs for details.
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
]

// Create a search index.
const index = tantivy.createInDir('.index', schema)

// On subsequent runs reopen the search index instead.
// const index = tantivy.openInDir(path)

// Have some docs that match the schema.
const docs = [
  { title: 'Hello, world', body: 'and it rusts down here' },
  { title: 'Sesame', body: 'the future is bright' }
]

// Open the index writer to add documents
index.writer()

// Add the documents.
index.addDocuments(docs)

// Commit the changes.
index.commit()

// Search!
const res = index.query('future')
console.log(res)
```
