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
    { title: 'Hello, moon', body: 'sea whale crates all over', url: 'a:2' },
    { title: 'Sesame', body: 'the future is brightly rusty', url: 'a:3' }
  ]

  const path = tempy.directory()

  const index = tantivy.createInDir(path, schema)

  index.writer()
  index.addDocuments(docs)
  index.commit()

  let res = index.query('hello')
  t.deepEqual(toUrl(res), ['a:1', 'a:2'], 'search 1 is correct')

  res = index.query('moon')
  t.deepEqual(toUrl(res), ['a:2'], 'search 2 is correct')

  res = index.query('future')
  t.deepEqual(toUrl(res), ['a:3'], 'search 3 is correct')

  t.end()

  function toUrl (res) {
    return res.map(d => d.url[0]).sort()
  }
})

