use crate::ast::prelude::*;
use crate::parser::prelude::Parser as ParserTrait;
use crate::shared::SharedMut;

impl ParserTrait {
    pub(crate) fn init_builtin_error_classes(&mut self) {
        let error_root = self.module.arena.intern_string(&self.interner, "Ошибка");
        self.module
            .arena
            .register_custom_type(&self.interner, "Ошибка");
        self.module
            .classes
            .entry(error_root)
            .or_insert_with(|| SharedMut::new(ClassDefinition::new(error_root, Span::default())));

        for class_name in [
            "ОшибкаПеременной",
            "ОшибкаФункции",
            "ОшибкаМетода",
            "ОшибкаТипа",
            "ОшибкаДеленияНаНоль",
            "ОшибкаОперации",
            "ОшибкаВводаВывода",
            "ОшибкаИмпорта",
            "Паника",
        ] {
            let symbol = self.module.arena.intern_string(&self.interner, class_name);
            self.module
                .arena
                .register_custom_type(&self.interner, class_name);
            self.module.classes.entry(symbol).or_insert_with(|| {
                SharedMut::new(ClassDefinition::new_with_base(
                    symbol,
                    Some(error_root),
                    Span::default(),
                ))
            });
        }
    }
}
