const tantivy = require('./lib/index.js')

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
index.openInDir(path, schema)

// tantivy.addIndex(path, schema)
docs.forEach(doc => index.addDocument(doc))
index.commit()

const res = index.query('and')
console.log(res)

