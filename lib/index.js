const tn = require('../native/index.node')

// const tn = Object.keys(tn_real).reduce((ret, key) => {
  // ret[key] = (...args) => {
    // console.log('CALL', key, args)
    // return tn_real[key](...args)
  // }
  // return ret
// }, {})

module.exports = {
  openInDir,
  createInDir,
  createInRam,
  tantivy: tn
}

function openInDir (path) {
  const handle = tn.index_open_in_dir(path)
  return new Index(handle)
}

function createInDir (path, schema) {
  const handle = tn.index_create_in_dir(path, JSON.stringify(schema))
  return new Index(handle)
}

function createInRam (schema) {
  const handle = tn.index_create_in_ram(schema)
  return new Index(handle)
}

class Index {
  constructor (handle) {
    this.handle = handle
  }

  query (q) {
    return tn.index_query(this.handle, q)
  }

  writer () {
    return tn.index_writer_open(this.handle)
  }

  addDocument (doc) {
    let transposed = Object.keys(doc).map(key => ({
      field: key,
      value: doc[key]
    }))
    return tn.index_writer_add_document(this.handle, transposed)
  }

  addDocuments (docs) {
    docs.forEach(doc => this.addDocument(doc))
  }

  commit () {
    return tn.index_writer_commit(this.handle)
  }
}

