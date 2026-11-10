use super::document::Document;

pub struct History {
    stack: Vec<Document>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new(60)
    }
}

impl History {
    pub fn new(limit: usize) -> Self {
        Self {
            stack: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub fn push(&mut self, doc: &Document) {
        self.stack.push(doc.clone());
        if self.stack.len() > self.limit {
            self.stack.remove(0);
        }
    }

    pub fn pop(&mut self) -> Option<Document> {
        self.stack.pop()
    }

    pub fn can_undo(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }
}
