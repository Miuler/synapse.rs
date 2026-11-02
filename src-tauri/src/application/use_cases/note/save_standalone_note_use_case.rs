use crate::application::dto::note::save_standalone_note_command::SaveStandaloneNoteCommand;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::note::note::Note;
use crate::domain::note::note_path::NoteRelativePath;

/// Guarda un archivo suelto, fuera de una bóveda.
#[derive(Clone)]
pub struct SaveStandaloneNoteUseCase<A: NoteRepository> {
    notes: A,
}

impl<A: NoteRepository> SaveStandaloneNoteUseCase<A> {
    pub fn new(notes: A) -> Self {
        Self { notes }
    }

    pub fn execute(&self, command: SaveStandaloneNoteCommand) -> Result<(), String> {
        let abs_path_str = command.path.to_string_lossy().to_string();
        let relative_path = NoteRelativePath::from_absolute_path(&abs_path_str)?;
        let note = Note::with_encoding(
            relative_path,
            abs_path_str,
            command.title,
            command.content,
            command.encoding,
        );
        self.notes.save_standalone_file(&command.path, &note)
    }
}
