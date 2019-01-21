const tantivy = require('../lib/index.js')
const tempy = require('tempy')
const tape = require('tape')

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

tape('basic indexing and search', t => {
  const docs = [
    { title: 'Hello, world', body: 'and it rusts down here', url: 'a:1' },
    { title: 'Hello, moon', body: 'crates all over', url: 'a:2' },
    { title: 'Sesame', body: 'the future is brightly broken', url: 'a:3' }
  ]

  const path = tempy.directory()

  tantivy.addIndex(path, schema)
  docs.forEach(doc => tantivy.addDocument(path, doc))

  let res = tantivy.query(path, 'hello')
  t.deepEqual(['a:1', 'a:2'], toUrl(res), 'search 1 is correct')

  res = tantivy.query(path, 'moon')
  t.deepEqual(['a:2'], toUrl(res), 'search 2 is correct')

  res = tantivy.query(path, 'future')
  t.deepEqual(['a:3'], toUrl(res), 'search 3 is correct')

  t.end()

  function toUrl (res) {
    return res.map(d => d.url[0]).sort()
  }
})

