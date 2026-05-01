use fxhash::FxHashMap;
use gen_lsp_types::Uri;

pub struct DocumentStorage {
    documents: FxHashMap<Uri, String>,
}

impl DocumentStorage {
    pub fn new() -> Self {
        DocumentStorage {
            documents: FxHashMap::default(),
        }
    }

    pub fn open(&mut self, uri: Uri, content: String) {
        self.documents.insert(uri, content);
    }

    pub fn change(&mut self, uri: Uri, change: String) {
        self.documents.insert(uri, change);
    }

    pub fn close(&mut self, uri: &Uri) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Uri) -> Option<&str> {
        self.documents.get(uri).map(|x| x.as_str())
    }
}
