use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, CallArgValue, RuntimeError, RuntimeMutex, RuntimeRwLock, RuntimeThread,
    SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, InterpreterFunctions};
use crate::{bail_runtime, define_constructor, define_method, runtime_error};
use string_interner::DefaultSymbol as Symbol;

fn spawn_thread(
    interp: &crate::interpreter::prelude::Interpreter,
    callable: Value,
    arguments: Vec<CallArgValue>,
    module_id: Symbol,
    span: Span,
) -> Result<RuntimeThread, RuntimeError> {
    match callable {
        Value::Function(function) => {
            let mut thread_interpreter = interp.fork_for_thread();
            let handle = std::thread::spawn(move || {
                let result = thread_interpreter
                    .call_function(function, arguments, module_id, span)
                    .map(|_| ());

                match result {
                    Err(RuntimeError::Return(..)) => Ok(()),
                    other => other,
                }?;

                thread_interpreter.join_background_threads(module_id, span)
            });
            Ok(RuntimeThread::new(handle))
        }
        Value::Builtin(builtin) => {
            let thread_interpreter = interp.fork_for_thread();
            let handle = std::thread::spawn(move || {
                builtin(&thread_interpreter, arguments, span).map(|_| ())
            });
            Ok(RuntimeThread::new(handle))
        }
        _ => bail_runtime!(TypeError, span, "Поток можно создать только из функции"),
    }
}

fn thread_from_args(
    interp: &crate::interpreter::prelude::Interpreter,
    args: &[CallArgValue],
    callable_index: usize,
    span: Span,
) -> Result<RuntimeThread, RuntimeError> {
    let Some(callable) = args.get_value(callable_index).cloned() else {
        return bail_runtime!(TypeError, span, "Поток ожидает функцию");
    };
    let arguments = args[callable_index + 1..].to_vec();
    spawn_thread(interp, callable, arguments, span.file_id, span)
}

fn wait_mutex_access(mutex: &RuntimeMutex, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*mutex.state;
    let mut state = state_lock
        .lock()
        .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;

    while state.owner.is_some() && state.owner != Some(current) {
        state = cvar
            .wait(state)
            .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;
    }

    Ok(())
}

fn lock_mutex(mutex: &RuntimeMutex, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*mutex.state;
    let mut state = state_lock
        .lock()
        .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;

    while state.owner.is_some() && state.owner != Some(current) {
        state = cvar
            .wait(state)
            .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;
    }

    state.owner = Some(current);
    state.depth += 1;
    Ok(())
}

fn unlock_mutex(mutex: &RuntimeMutex, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*mutex.state;
    let mut state = state_lock
        .lock()
        .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;

    if state.owner != Some(current) {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Нельзя разблокировать мьютекс из потока, который его не блокировал"
        );
    }

    state.depth -= 1;
    if state.depth == 0 {
        state.owner = None;
        cvar.notify_one();
    }
    Ok(())
}

fn can_read_rw(
    state: &crate::interpreter::prelude::RwLockState,
    current: std::thread::ThreadId,
) -> bool {
    state.writer.is_none() || state.writer == Some(current)
}

fn can_write_rw(
    state: &crate::interpreter::prelude::RwLockState,
    current: std::thread::ThreadId,
) -> bool {
    let other_readers = state
        .readers
        .iter()
        .any(|(reader, count)| *reader != current && *count > 0);
    (state.writer.is_none() || state.writer == Some(current)) && !other_readers
}

fn wait_rw_read(lock: &RuntimeRwLock, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*lock.state;
    let mut state = state_lock.lock().map_err(|_| {
        runtime_error!(
            InvalidOperation,
            span,
            "Блокировка чтения-записи повреждена"
        )
    })?;

    while !can_read_rw(&state, current) {
        state = cvar.wait(state).map_err(|_| {
            runtime_error!(
                InvalidOperation,
                span,
                "Блокировка чтения-записи повреждена"
            )
        })?;
    }

    Ok(())
}

fn wait_rw_write(lock: &RuntimeRwLock, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*lock.state;
    let mut state = state_lock.lock().map_err(|_| {
        runtime_error!(
            InvalidOperation,
            span,
            "Блокировка чтения-записи повреждена"
        )
    })?;

    while !can_write_rw(&state, current) {
        state = cvar.wait(state).map_err(|_| {
            runtime_error!(
                InvalidOperation,
                span,
                "Блокировка чтения-записи повреждена"
            )
        })?;
    }

    Ok(())
}

fn lock_rw_read(lock: &RuntimeRwLock, span: Span) -> Result<(), RuntimeError> {
    wait_rw_read(lock, span)?;
    let current = std::thread::current().id();
    let (state_lock, _) = &*lock.state;
    let mut state = state_lock.lock().map_err(|_| {
        runtime_error!(
            InvalidOperation,
            span,
            "Блокировка чтения-записи повреждена"
        )
    })?;
    *state.readers.entry(current).or_insert(0) += 1;
    Ok(())
}

fn unlock_rw_read(lock: &RuntimeRwLock, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*lock.state;
    let mut state = state_lock.lock().map_err(|_| {
        runtime_error!(
            InvalidOperation,
            span,
            "Блокировка чтения-записи повреждена"
        )
    })?;

    let Some(count) = state.readers.get_mut(&current) else {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Нельзя снять блокировку чтения из потока, который ее не ставил"
        );
    };

    *count -= 1;
    if *count == 0 {
        state.readers.remove(&current);
    }
    cvar.notify_all();
    Ok(())
}

fn lock_rw_write(lock: &RuntimeRwLock, span: Span) -> Result<(), RuntimeError> {
    wait_rw_write(lock, span)?;
    let current = std::thread::current().id();
    let (state_lock, _) = &*lock.state;
    let mut state = state_lock.lock().map_err(|_| {
        runtime_error!(
            InvalidOperation,
            span,
            "Блокировка чтения-записи повреждена"
        )
    })?;
    state.writer = Some(current);
    state.writer_depth += 1;
    Ok(())
}

fn unlock_rw_write(lock: &RuntimeRwLock, span: Span) -> Result<(), RuntimeError> {
    let current = std::thread::current().id();
    let (state_lock, cvar) = &*lock.state;
    let mut state = state_lock.lock().map_err(|_| {
        runtime_error!(
            InvalidOperation,
            span,
            "Блокировка чтения-записи повреждена"
        )
    })?;

    if state.writer != Some(current) {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Нельзя снять блокировку записи из потока, который ее не ставил"
        );
    }

    state.writer_depth -= 1;
    if state.writer_depth == 0 {
        state.writer = None;
        cvar.notify_all();
    }
    Ok(())
}

pub fn setup_thread_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner
        .write(|i| i.get_or_intern(crate::builtins::catalog::class::THREAD.names.canonical));
    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, span) {
        let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) else {
            return bail_runtime!(TypeError, span, "Ожидался объект Поток");
        };

        if args.len() == 1 {
            return Ok(Value::Empty);
        }

        let thread = thread_from_args(interp, &args, 1, span)?;
        let data_sym = interp.intern_string("__data");
        instance.write(|i| {
            i.field_values.insert(data_sym, Value::Thread(thread));
        });
        Ok(Value::Empty)
    });

    define_method!(class_def, interner, @static crate::builtins::catalog::method::CREATE.canonical => (interp, args, span) {
        let thread = thread_from_args(interp, &args, 0, span)?;
        Ok(Value::Thread(thread))
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::JOIN_THREAD.canonical => (interp, args, span) {
        if let Some(Value::Thread(thread_value)) = CallArgListExt::first_value(&args) {
            interp.join_thread_handle(thread_value, span)
        } else {
            bail_runtime!(TypeError, span, "Ожидался Поток")
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_mutex_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name =
        interner.write(|i| i.get_or_intern(crate::builtins::catalog::class::MUTEX.names.canonical));
    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, span) {
        let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) else {
            return bail_runtime!(TypeError, span, "Ожидался объект Мьютекс");
        };
        let initial = CallArgListExt::get_value(&args, 1)
            .cloned()
            .unwrap_or(Value::Empty);
        let data_sym = interp.intern_string("__data");
        instance.write(|i| {
            i.field_values.insert(data_sym, Value::Mutex(RuntimeMutex::new(initial)));
        });
        Ok(Value::Empty)
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::LOCK.canonical => (_, args, span) {
        if let Some(Value::Mutex(mutex)) = CallArgListExt::first_value(&args) {
            lock_mutex(mutex, span)?;
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидался Мьютекс")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::UNLOCK.canonical => (_, args, span) {
        if let Some(Value::Mutex(mutex)) = CallArgListExt::first_value(&args) {
            unlock_mutex(mutex, span)?;
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидался Мьютекс")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::READ.canonical => (_, args, span) {
        if let Some(Value::Mutex(mutex)) = CallArgListExt::first_value(&args) {
            wait_mutex_access(mutex, span)?;
            let guard = mutex
                .value
                .lock()
                .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;
            Ok(guard.clone())
        } else {
            bail_runtime!(TypeError, span, "Ожидался Мьютекс")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::WRITE.canonical => (_, args, span) {
        if let (Some(Value::Mutex(mutex)), Some(new_value)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            wait_mutex_access(mutex, span)?;
            let mut guard = mutex
                .value
                .lock()
                .map_err(|_| runtime_error!(InvalidOperation, span, "Мьютекс поврежден"))?;
            *guard = new_value.clone();
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Использование: мьютекс.записать(значение)")
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_rwlock_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner
        .write(|i| i.get_or_intern(crate::builtins::catalog::class::RWLOCK.names.canonical));
    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, span) {
        let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) else {
            return bail_runtime!(TypeError, span, "Ожидался объект БлокировкаЧтенияЗаписи");
        };
        let initial = CallArgListExt::get_value(&args, 1)
            .cloned()
            .unwrap_or(Value::Empty);
        let data_sym = interp.intern_string("__data");
        instance.write(|i| {
            i.field_values.insert(data_sym, Value::RwLock(RuntimeRwLock::new(initial)));
        });
        Ok(Value::Empty)
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::WRITE_LOCK.canonical => (_, args, span) {
        if let Some(Value::RwLock(lock)) = CallArgListExt::first_value(&args) {
            lock_rw_write(lock, span)?;
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидалась БлокировкаЧтенияЗаписи")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::WRITE_UNLOCK.canonical => (_, args, span) {
        if let Some(Value::RwLock(lock)) = CallArgListExt::first_value(&args) {
            unlock_rw_write(lock, span)?;
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидалась БлокировкаЧтенияЗаписи")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::READ_LOCK.canonical => (_, args, span) {
        if let Some(Value::RwLock(lock)) = CallArgListExt::first_value(&args) {
            lock_rw_read(lock, span)?;
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидалась БлокировкаЧтенияЗаписи")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::READ_UNLOCK.canonical => (_, args, span) {
        if let Some(Value::RwLock(lock)) = CallArgListExt::first_value(&args) {
            unlock_rw_read(lock, span)?;
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидалась БлокировкаЧтенияЗаписи")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::READ.canonical => (_, args, span) {
        if let Some(Value::RwLock(lock)) = CallArgListExt::first_value(&args) {
            wait_rw_read(lock, span)?;
            let guard = lock.value.read().map_err(|_| {
                runtime_error!(InvalidOperation, span, "Блокировка чтения-записи повреждена")
            })?;
            Ok(guard.clone())
        } else {
            bail_runtime!(TypeError, span, "Ожидалась БлокировкаЧтенияЗаписи")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::WRITE.canonical => (_, args, span) {
        if let (Some(Value::RwLock(lock)), Some(new_value)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            wait_rw_write(lock, span)?;
            let mut guard = lock.value.write().map_err(|_| {
                runtime_error!(InvalidOperation, span, "Блокировка чтения-записи повреждена")
            })?;
            *guard = new_value.clone();
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Использование: блокировка.записать(значение)")
        }
    });

    (name, SharedMut::new(class_def))
}
