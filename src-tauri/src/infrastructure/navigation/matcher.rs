use super::model::{NoteId, NoteMeta};
use dashmap::DashMap;
use nucleo::{Nucleo, Utf32String};

pub fn populate_nucleo_from_dashmap(map: &DashMap<NoteId, NoteMeta>, nucleo: &mut Nucleo<NoteId>) {
    let injector = nucleo.injector();
    for item in map.iter() {
        let id = *item.key();
        let title = item.value().title.clone();
        let path = item.value().path.clone();
        let tags = item.value().tags.clone();

        injector.push(id, move |_target, cols| {
            let tags_len: usize = tags.iter().map(|t| t.len() + 2).sum();
            let mut s = String::with_capacity(title.len() + 1 + path.len() + tags_len);
            s.push_str(title.as_str());
            if title.as_str() != path.as_str() {
                s.push(' ');
                s.push_str(path.as_str());
            }
            for tag in &tags {
                s.push(' ');
                s.push('#');
                s.push_str(tag.as_str());
                s.push(' ');
                s.push_str(tag.as_str());
            }
            cols[0] = Utf32String::from(s.as_str());
        });
    }
}
