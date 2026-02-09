use crate::interpreter::prelude::Interpreter;

mod array;
mod bool;
mod common;
mod dict;
mod file;
mod float;
mod io;
mod list;
mod number;
mod text;
mod system;
mod terminal;
mod datetime;

impl Interpreter {
    pub fn define_builtins(&mut self) {
        let interner = self.interner.clone();

        list::setup_list_func(self, &interner);
        array::setup_array_func(self, &interner);
        dict::setup_dict_func(self, &interner);
        text::setup_text_func(self, &interner);
        number::setup_number_func(self, &interner);
        float::setup_float_func(self, &interner);
        bool::setup_bool_func(self, &interner);
        io::setup_io_func(self, &interner);
        common::setup_type_func(self, &interner);
        common::setup_is_instance_func(self, &interner);

        let (string_name, string_class) = text::setup_text_class(&interner);
        self.std_classes.insert(string_name, string_class);
        let (list_name, list_class) = list::setup_list_class(&interner);
        self.std_classes.insert(list_name, list_class);
        let (array_name, array_class) = array::setup_array_class(&interner);
        self.std_classes.insert(array_name, array_class);
        let (dict_name, dict_class) = dict::setup_dict_class(&interner);
        self.std_classes.insert(dict_name, dict_class);
        let (file_name, file_class) = file::setup_file_class(&interner);
        self.std_classes.insert(file_name, file_class);
        let (system_name, system_class) = system::setup_system_class(&interner);
        self.std_classes.insert(system_name, system_class);
        let (color_name, color_class) = terminal::setup_terminal_class(&interner);
        self.std_classes.insert(color_name, color_class);
        let (datetime_name, datetime_class) = datetime::setup_datetime_class(&interner);
        self.std_classes.insert(datetime_name, datetime_class);
    }
}
