const tantivy = require('../native/index.node')

class Index extends tantivy.Index {
  addDocument (doc) {
    let field_values = Object.keys(doc).reduce((list, field) => {
      list.push({ field, value: doc[field ] })
      return list
    }, [])
    super.addDocument({ field_values })
  }
}

module.exports = () => new Index()

