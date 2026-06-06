use crate::interpreter::prelude::Interpreter;
use crate::shared::SharedMut;
use crate::{ast::prelude::ClassDefinition, interpreter::prelude::SharedInterner};
use string_interner::DefaultSymbol as Symbol;

mod array;
mod bool;
pub mod catalog;
mod common;
mod datetime;
mod dict;
mod file;
mod float;
mod io;
pub(crate) mod iterator;
mod json;
mod list;
pub(crate) mod macros;
mod number;
mod regex;
mod system;
mod terminal;
mod text;
mod thread;

impl Interpreter {
    pub fn define_builtins(&mut self) {
        let interner = self.interner.clone();

        list::setup_list_func(self, &interner);
        array::setup_array_func(self, &interner);
        dict::setup_dict_func(self, &interner);
        iterator::setup_iterator_func(self, &interner);
        text::setup_text_func(self, &interner);
        number::setup_number_func(self, &interner);
        float::setup_float_func(self, &interner);
        bool::setup_bool_func(self, &interner);
        json::setup_json_funcs(self, &interner);
        io::setup_io_func(self, &interner);
        common::setup_type_func(self, &interner);
        common::setup_is_instance_func(self, &interner);
        regex::setup_regex_func(self, &interner);

        let (string_name, string_class) = text::setup_text_class(&interner);
        self.register_std_class_aliases(&interner, string_name, string_class);
        let (list_name, list_class) = list::setup_list_class(&interner);
        self.register_std_class_aliases(&interner, list_name, list_class);
        let (array_name, array_class) = array::setup_array_class(&interner);
        self.register_std_class_aliases(&interner, array_name, array_class);
        let (dict_name, dict_class) = dict::setup_dict_class(&interner);
        self.register_std_class_aliases(&interner, dict_name, dict_class);
        let (iterator_name, iterator_class) = iterator::setup_iterator_class(&interner);
        self.register_std_class_aliases(&interner, iterator_name, iterator_class);
        let (file_name, file_class) = file::setup_file_class(&interner);
        self.register_std_class_aliases(&interner, file_name, file_class);
        let (system_name, system_class) = system::setup_system_class(&interner);
        self.register_std_class_aliases(&interner, system_name, system_class);
        let (terminal_name, terminal_class) = terminal::setup_terminal_class(&interner);
        self.register_std_class_aliases(&interner, terminal_name, terminal_class);
        let (datetime_name, datetime_class) = datetime::setup_datetime_class(&interner);
        self.register_std_class_aliases(&interner, datetime_name, datetime_class);
        let (regex_name, regex_class) = regex::setup_regex_class(&interner);
        self.register_std_class_aliases(&interner, regex_name, regex_class);
        let (thread_name, thread_class) = thread::setup_thread_class(&interner);
        self.register_std_class_aliases(&interner, thread_name, thread_class);
        let (mutex_name, mutex_class) = thread::setup_mutex_class(&interner);
        self.register_std_class_aliases(&interner, mutex_name, mutex_class);
        let (rwlock_name, rwlock_class) = thread::setup_rwlock_class(&interner);
        self.register_std_class_aliases(&interner, rwlock_name, rwlock_class);
    }

    fn register_std_class_aliases(
        &mut self,
        interner: &SharedInterner,
        canonical: Symbol,
        class: SharedMut<ClassDefinition>,
    ) {
        let canonical_name = interner
            .read(|i| i.resolve(canonical).map(str::to_owned))
            .unwrap_or_default();
        let aliases = catalog::class_names(&canonical_name);

        if aliases.is_empty() {
            self.std_classes.insert(canonical, class);
            return;
        }

        for alias in aliases {
            self.std_classes
                .insert(interner.write(|i| i.get_or_intern(alias)), class.clone());
        }
    }
}
