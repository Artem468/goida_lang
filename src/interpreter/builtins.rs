use crate::Arc;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, Value};
use crate::traits::prelude::CoreOperations;
use crate::{define_builtin, setup_builtins};
use std::io;
use std::io::Write;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;


setup_builtins!(interpreter, {
    "печать" (arguments) {
        let sep =  " ";
        let end = "\n";
        print!(
            "{}{}",
            arguments
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(sep),
            end
        );
    }

    "ввод" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'ввод' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        print!("{}", arguments[0]);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        Ok(Value::Text(input.to_string()))
    }

     "число" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'число' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: i64 = arguments[0].clone().try_into()?;
        Ok(Value::Number(n))
    }

    "дробь" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: f64 = arguments[0].clone().try_into()?;
        Ok(Value::Float(n))
    }

    "текст" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: String = arguments[0].clone().try_into()?;
        Ok(Value::Text(n))
    }

    "логический" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: bool = arguments[0].clone().try_into()?;
        Ok(Value::Boolean(n))
    }

    "тип" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'тип' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let val = arguments.get(0).ok_or_else(|| RuntimeError::InvalidOperation("Не передан объект".into()))?;
        match val
         {
            Value::Number(_) => Ok(Value::Text("число".to_string())),
            Value::Float(_) => Ok(Value::Text("дробь".to_string())),
            Value::Text(_) => Ok(Value::Text("текст".to_string())),
            Value::Boolean(_) => Ok(Value::Text("логический".to_string())),
            Value::Object(obj) => Ok(
                Value::Text(
                     format!(
                         "объект \"{}\"",
                         interpreter
                         .resolve_symbol(obj.borrow().class_name)
                         .ok_or_else(|| RuntimeError::InvalidOperation("Тип не найден".into()))?.to_string()
                     )
                 )

            ),
            Value::Function(obj) => Ok(
                Value::Text(
                    format!(
                         "функция \"{}\"",
                         interpreter
                        .resolve_symbol(obj.name)
                        .ok_or_else(|| RuntimeError::InvalidOperation("Тип не найден".into()))?.to_string()
                    )
                )
            ),
            Value::Builtin(_) => {
                Ok(Value::Text("встроенная функция".to_string()))
            },
            Value::Module(sym) => {
                Ok(Value::Text(
                    format!(
                        "модуль \"{}\"",
                        interpreter
                        .resolve_symbol(*sym)
                        .ok_or_else(|| RuntimeError::InvalidOperation("Модуль не найден".into()))?
                    )
                ))
            },
            Value::List(_) => {
                Ok(Value::Text("список".to_string()))
            },
            Value::Array(_) => {
                Ok(Value::Text("массив".to_string()))
            },
            Value::Dict(_) => {
                Ok(Value::Text("словарь".to_string()))
            },
            Value::Empty => {
                Ok(Value::Text("пустота".to_string()))
            }
        }
    }

    "список" (arguments) -> Result<Value, RuntimeError> {
        use std::rc::Rc;
        use std::cell::RefCell;
        Ok(Value::List(Rc::new(RefCell::new(arguments))))
    }

    "массив" (arguments) -> Result<Value, RuntimeError> {
        use std::rc::Rc;
        Ok(Value::Array(Rc::new(arguments)))
    }

    "словарь" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() % 2 != 0 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'словарь' ожидает четное количество аргументов (пары ключ-значение)".to_string()
            ));
        }

        let mut dict = HashMap::new();
        for i in (0..arguments.len()).step_by(2) {
            let key = match &arguments[i] {
                Value::Text(s) => s.clone(),
                v => v.to_string(),
            };
            let value = arguments[i + 1].clone();
            dict.insert(key, value);
        }

        Ok(Value::Dict(Rc::new(RefCell::new(dict))))
    }

    "добавить" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() < 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'добавить' ожидает минимум 2 аргумента (коллекция и элемент)".to_string()
            ));
        }

        match &arguments[0] {
            Value::List(list) => {
                let mut list_ref = list.borrow_mut();
                for i in 1..arguments.len() {
                    list_ref.push(arguments[i].clone());
                }
                Ok(Value::Empty)
            }
            Value::Dict(dict) => {
                if arguments.len() < 3 || arguments.len() % 2 == 0 {
                    return Err(RuntimeError::InvalidOperation(
                        "Функция 'добавить' для словаря требует ключ и значение".to_string()
                    ));
                }
                let mut dict_ref = dict.borrow_mut();
                for i in (1..arguments.len()).step_by(2) {
                    let key = match &arguments[i] {
                        Value::Text(s) => s.clone(),
                        v => v.to_string(),
                    };
                    let value = arguments[i + 1].clone();
                    dict_ref.insert(key, value);
                }
                Ok(Value::Empty)
            }
            Value::Array(_) => Err(RuntimeError::InvalidOperation(
                "Массив неизменяемый, используйте список для добавления элементов".to_string()
            )),
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'добавить' работает только со списками и словарями".to_string()
            )),
        }
    }

    "получить" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'получить' ожидает 2 аргумента (коллекция и индекс/ключ)".to_string()
            ));
        }

        match &arguments[0] {
            Value::List(list) => {
                let idx: i64 = arguments[1].clone().try_into()?;
                let list_ref = list.borrow();
                let index = if idx < 0 {
                    (list_ref.len() as i64 + idx) as usize
                } else {
                    idx as usize
                };
                Ok(list_ref.get(index).cloned().unwrap_or(Value::Empty))
            }
            Value::Array(array) => {
                let idx: i64 = arguments[1].clone().try_into()?;
                let index = if idx < 0 {
                    (array.len() as i64 + idx) as usize
                } else {
                    idx as usize
                };
                Ok(array.get(index).cloned().unwrap_or(Value::Empty))
            }
            Value::Dict(dict) => {
                let key = match &arguments[1] {
                    Value::Text(s) => s.clone(),
                    v => v.to_string(),
                };
                let dict_ref = dict.borrow();
                Ok(dict_ref.get(&key).cloned().unwrap_or(Value::Empty))
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'получить' работает только с коллекциями".to_string()
            )),
        }
    }

    "удалить" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'удалить' ожидает 2 аргумента (коллекция и индекс/ключ)".to_string()
            ));
        }

        match &arguments[0] {
            Value::List(list) => {
                let idx: i64 = arguments[1].clone().try_into()?;
                let mut list_ref = list.borrow_mut();
                let index = if idx < 0 {
                    (list_ref.len() as i64 + idx) as usize
                } else {
                    idx as usize
                };
                if index < list_ref.len() {
                    Ok(list_ref.remove(index))
                } else {
                    Ok(Value::Empty)
                }
            }
            Value::Dict(dict) => {
                let key = match &arguments[1] {
                    Value::Text(s) => s.clone(),
                    v => v.to_string(),
                };
                let mut dict_ref = dict.borrow_mut();
                Ok(dict_ref.remove(&key).unwrap_or(Value::Empty))
            }
            Value::Array(_) => Err(RuntimeError::InvalidOperation(
                "Массив неизменяемый, используйте список для удаления элементов".to_string()
            )),
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'удалить' работает только со списками и словарями".to_string()
            )),
        }
    }

    "изменить" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 3 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'изменить' ожидает 3 аргумента (коллекция, индекс/ключ, значение)".to_string()
            ));
        }

        match &arguments[0] {
            Value::List(list) => {
                let idx: i64 = arguments[1].clone().try_into()?;
                let mut list_ref = list.borrow_mut();
                let index = if idx < 0 {
                    (list_ref.len() as i64 + idx) as usize
                } else {
                    idx as usize
                };
                if index < list_ref.len() {
                    let old = list_ref[index].clone();
                    list_ref[index] = arguments[2].clone();
                    Ok(old)
                } else {
                    Err(RuntimeError::InvalidOperation("Индекс за границами списка".to_string()))
                }
            }
            Value::Dict(dict) => {
                let key = match &arguments[1] {
                    Value::Text(s) => s.clone(),
                    v => v.to_string(),
                };
                let mut dict_ref = dict.borrow_mut();
                let old = dict_ref.get(&key).cloned();
                dict_ref.insert(key, arguments[2].clone());
                Ok(old.unwrap_or(Value::Empty))
            }
            Value::Array(_) => Err(RuntimeError::InvalidOperation(
                "Массив неизменяемый, используйте список для изменения элементов".to_string()
            )),
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'изменить' работает только со списками и словарями".to_string()
            )),
        }
    }

    "длина" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'длина' ожидает 1 аргумент (коллекция)".to_string()
            ));
        }

        match &arguments[0] {
            Value::List(list) => Ok(Value::Number(list.borrow().len() as i64)),
            Value::Array(array) => Ok(Value::Number(array.len() as i64)),
            Value::Dict(dict) => Ok(Value::Number(dict.borrow().len() as i64)),
            Value::Text(text) => Ok(Value::Number(text.len() as i64)),
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'длина' работает только с коллекциями и текстом".to_string()
            )),
        }
    }

    "ключи" (arguments) -> Result<Value, RuntimeError> {
        use std::rc::Rc;
        use std::cell::RefCell;
        
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'ключи' ожидает 1 аргумент (словарь)".to_string()
            ));
        }

        match &arguments[0] {
            Value::Dict(dict) => {
                let dict_ref = dict.borrow();
                let keys: Vec<Value> = dict_ref.keys()
                    .map(|k| Value::Text(k.clone()))
                    .collect();
                Ok(Value::List(Rc::new(RefCell::new(keys))))
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'ключи' работает только со словарями".to_string()
            )),
        }
    }

    "значения" (arguments) -> Result<Value, RuntimeError> {
        use std::rc::Rc;
        use std::cell::RefCell;
        
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'значения' ожидает 1 аргумент (словарь)".to_string()
            ));
        }

        match &arguments[0] {
            Value::Dict(dict) => {
                let dict_ref = dict.borrow();
                let values: Vec<Value> = dict_ref.values().cloned().collect();
                Ok(Value::List(Rc::new(RefCell::new(values))))
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'значения' работает только со словарями".to_string()
            )),
        }
    }
});
