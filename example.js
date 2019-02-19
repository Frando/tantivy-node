const tantivy = require('.')
const mkdirp = require('mkdirp')
const fs = require('fs')
const p = require('path')

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

if (!fs.existsSync(p.join(path, 'meta.json'))) {
  mkdirp(path, create)
} else {
  open()
}

function create (err) {
  console.log(`create new index at ${path}`)
  if (err) exit(err)
  const index = tantivy.createInDir(path, schema)
  start(index)
}

function open () {
  console.log(`open index at ${path}`)
  const index = tantivy.openInDir(path)
  start(index)
}

function start (index) {
  let docs = [
    {
      body: 'Now for real.',
      title: 'Hello, world!'
    },
    {
      body: 'The future',
      title: 'Looks like a bright bright world'
    },
  ]
  index.writer()
  index.addDocuments(docs)
  index.commit()

  let res = index.query('world')
  console.log(res)
}

