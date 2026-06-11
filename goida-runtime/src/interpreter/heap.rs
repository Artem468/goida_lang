use crate::interpreter::structs::{ClassInstance, Value};
use crate::shared::SharedMut;
use goida_model::WeakSharedMut;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock, Weak};

pub type ObjectId = u64;

const INITIAL_COLLECTION_THRESHOLD: usize = 256;
const COLLECTION_GROWTH_FACTOR: usize = 2;

#[derive(Debug)]
pub struct ObjectHeap {
    state: Mutex<HeapState>,
}

#[derive(Debug)]
struct HeapState {
    next_id: ObjectId,
    collection_threshold: usize,
    objects: HashMap<usize, HeapEntry>,
}

#[derive(Debug)]
struct HeapEntry {
    id: ObjectId,
    object: WeakObject,
}

#[derive(Clone, Debug)]
enum WeakObject {
    Object(WeakSharedMut<ClassInstance>),
    List(WeakSharedMut<Vec<Value>>),
    Dict(WeakSharedMut<HashMap<String, Value>>),
    Mutex(Weak<Mutex<Value>>),
    RwLock(Weak<RwLock<Value>>),
}

#[derive(Clone, Debug)]
enum LiveObject {
    Object(SharedMut<ClassInstance>),
    List(SharedMut<Vec<Value>>),
    Dict(SharedMut<HashMap<String, Value>>),
    Mutex(Arc<Mutex<Value>>),
    RwLock(Arc<RwLock<Value>>),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CollectionStats {
    pub tracked: usize,
    pub collected: usize,
}

impl ObjectHeap {
    pub fn adopt(&self, value: &Value) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut visited = HashSet::new();
        adopt_value(&mut state, value, &mut visited);
    }

    pub fn collect_if_needed(&self) -> Option<CollectionStats> {
        let should_collect = {
            let state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.objects.len() >= state.collection_threshold
        };
        if should_collect {
            Some(self.collect_cycles())
        } else {
            None
        }
    }

    pub fn object_id(&self, value: &Value) -> Option<ObjectId> {
        let identity = managed_identity(value)?;
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .objects
            .get(&identity)
            .map(|entry| entry.id)
    }

    pub fn collect_cycles(&self) -> CollectionStats {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state
            .objects
            .retain(|_, entry| entry.object.upgrade().is_some());

        let live = state
            .objects
            .iter()
            .filter_map(|(identity, entry)| {
                entry.object.upgrade().map(|object| (*identity, object))
            })
            .collect::<HashMap<_, _>>();
        let tracked = live.len();
        let mut incoming = HashMap::<usize, usize>::new();
        let mut edges = HashMap::<usize, Vec<usize>>::new();

        for (identity, object) in &live {
            let mut targets = Vec::new();
            object.trace(|target| {
                if live.contains_key(&target) {
                    *incoming.entry(target).or_default() += 1;
                    targets.push(target);
                }
            });
            edges.insert(*identity, targets);
        }

        let mut reachable = HashSet::new();
        let mut pending = live
            .iter()
            .filter_map(|(identity, object)| {
                let external = object
                    .strong_count()
                    .saturating_sub(1)
                    .saturating_sub(incoming.get(identity).copied().unwrap_or_default());
                (external > 0).then_some(*identity)
            })
            .collect::<Vec<_>>();

        while let Some(identity) = pending.pop() {
            if !reachable.insert(identity) {
                continue;
            }
            if let Some(targets) = edges.get(&identity) {
                pending.extend(targets);
            }
        }

        let unreachable = live
            .iter()
            .filter_map(|(identity, object)| {
                (!reachable.contains(identity)).then_some((*identity, object.clone()))
            })
            .collect::<Vec<_>>();
        for (_, object) in &unreachable {
            object.clear();
        }
        for (identity, _) in &unreachable {
            state.objects.remove(identity);
        }
        state.collection_threshold = tracked
            .saturating_sub(unreachable.len())
            .saturating_mul(COLLECTION_GROWTH_FACTOR)
            .max(INITIAL_COLLECTION_THRESHOLD);

        CollectionStats {
            tracked,
            collected: unreachable.len(),
        }
    }

    #[cfg(test)]
    fn tracked_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .objects
            .len()
    }
}

impl Default for ObjectHeap {
    fn default() -> Self {
        Self {
            state: Mutex::new(HeapState {
                next_id: 0,
                collection_threshold: INITIAL_COLLECTION_THRESHOLD,
                objects: HashMap::new(),
            }),
        }
    }
}

impl WeakObject {
    fn upgrade(&self) -> Option<LiveObject> {
        match self {
            Self::Object(value) => value.upgrade().map(LiveObject::Object),
            Self::List(value) => value.upgrade().map(LiveObject::List),
            Self::Dict(value) => value.upgrade().map(LiveObject::Dict),
            Self::Mutex(value) => value.upgrade().map(LiveObject::Mutex),
            Self::RwLock(value) => value.upgrade().map(LiveObject::RwLock),
        }
    }
}

impl LiveObject {
    fn strong_count(&self) -> usize {
        match self {
            Self::Object(value) => value.strong_count(),
            Self::List(value) => value.strong_count(),
            Self::Dict(value) => value.strong_count(),
            Self::Mutex(value) => Arc::strong_count(value),
            Self::RwLock(value) => Arc::strong_count(value),
        }
    }

    fn trace(&self, mut visit: impl FnMut(usize)) {
        match self {
            Self::Object(value) => value.read(|value| {
                for child in value.field_values.values() {
                    trace_value(child, &mut visit);
                }
            }),
            Self::List(value) => value.read(|value| {
                for child in value {
                    trace_value(child, &mut visit);
                }
            }),
            Self::Dict(value) => value.read(|value| {
                for child in value.values() {
                    trace_value(child, &mut visit);
                }
            }),
            Self::Mutex(value) => {
                let value = value
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                trace_value(&value, &mut visit);
            }
            Self::RwLock(value) => {
                let value = value
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                trace_value(&value, &mut visit);
            }
        }
    }

    fn clear(&self) {
        match self {
            Self::Object(value) => value.write(|value| value.field_values.clear()),
            Self::List(value) => value.write(Vec::clear),
            Self::Dict(value) => value.write(HashMap::clear),
            Self::Mutex(value) => {
                *value
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Value::Empty;
            }
            Self::RwLock(value) => {
                *value
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Value::Empty;
            }
        }
    }
}

fn adopt_value(state: &mut HeapState, value: &Value, visited: &mut HashSet<usize>) {
    if let Some((identity, object)) = weak_object(value) {
        if !visited.insert(identity) {
            return;
        }
        let needs_registration = state
            .objects
            .get(&identity)
            .is_none_or(|entry| entry.object.upgrade().is_none());
        if needs_registration {
            let id = state.next_id;
            state.next_id = state
                .next_id
                .checked_add(1)
                .expect("managed object ID space exhausted");
            state.objects.insert(identity, HeapEntry { id, object });
        }
    }

    trace_nested_values(value, |child| adopt_value(state, child, visited));
}

fn trace_nested_values(value: &Value, mut visit: impl FnMut(&Value)) {
    match value {
        Value::Object(value) => value.read(|value| {
            for child in value.field_values.values() {
                visit(child);
            }
        }),
        Value::List(value) => value.read(|value| {
            for child in value {
                visit(child);
            }
        }),
        Value::Dict(value) => value.read(|value| {
            for child in value.values() {
                visit(child);
            }
        }),
        Value::Array(value) => {
            for child in value.iter() {
                visit(child);
            }
        }
        Value::Iterator(value) => {
            for child in value.source.iter() {
                visit(child);
            }
            for step in value.steps.iter() {
                match step {
                    crate::interpreter::structs::IteratorStep::Map(child)
                    | crate::interpreter::structs::IteratorStep::Filter(child) => visit(child),
                }
            }
        }
        Value::Mutex(value) => {
            let guard = value
                .value
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            visit(&guard);
        }
        Value::RwLock(value) => {
            let guard = value
                .value
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            visit(&guard);
        }
        _ => {}
    }
}

fn trace_value(value: &Value, visit: &mut impl FnMut(usize)) {
    if let Some(identity) = managed_identity(value) {
        visit(identity);
        return;
    }
    trace_nested_values(value, |child| trace_value(child, visit));
}

fn managed_identity(value: &Value) -> Option<usize> {
    match value {
        Value::Object(value) => Some(value.identity()),
        Value::List(value) => Some(value.identity()),
        Value::Dict(value) => Some(value.identity()),
        Value::Mutex(value) => Some(Arc::as_ptr(&value.value) as usize),
        Value::RwLock(value) => Some(Arc::as_ptr(&value.value) as usize),
        _ => None,
    }
}

fn weak_object(value: &Value) -> Option<(usize, WeakObject)> {
    match value {
        Value::Object(value) => Some((value.identity(), WeakObject::Object(value.downgrade()))),
        Value::List(value) => Some((value.identity(), WeakObject::List(value.downgrade()))),
        Value::Dict(value) => Some((value.identity(), WeakObject::Dict(value.downgrade()))),
        Value::Mutex(value) => Some((
            Arc::as_ptr(&value.value) as usize,
            WeakObject::Mutex(Arc::downgrade(&value.value)),
        )),
        Value::RwLock(value) => Some((
            Arc::as_ptr(&value.value) as usize,
            WeakObject::RwLock(Arc::downgrade(&value.value)),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::registry::BUILTINS;
    use crate::interpreter::prelude::{CallArgValue, Interpreter};
    use crate::interpreter::structs::{RuntimeClassDefinition, RuntimeMutex, RuntimeRwLock};
    use crate::traits::runtime::CoreOperations;
    use string_interner::{DefaultSymbol, Symbol as _};

    #[test]
    fn collects_self_referencing_list() {
        let heap = ObjectHeap::default();
        let list = SharedMut::new(Vec::new());
        let value = Value::List(list.clone());
        heap.adopt(&value);
        list.write(|items| items.push(value.clone()));
        drop(value);
        drop(list);

        let stats = heap.collect_cycles();

        assert_eq!(stats.collected, 1);
        assert_eq!(heap.tracked_count(), 0);
    }

    #[test]
    fn preserves_cycle_reachable_from_external_value() {
        let heap = ObjectHeap::default();
        let list = SharedMut::new(Vec::new());
        let value = Value::List(list.clone());
        heap.adopt(&value);
        list.write(|items| items.push(value.clone()));

        assert_eq!(heap.collect_cycles().collected, 0);
        assert_eq!(list.read(Vec::len), 1);
    }

    #[test]
    fn collects_mutually_referencing_dicts() {
        let heap = ObjectHeap::default();
        let left = Value::Dict(SharedMut::new(HashMap::new()));
        let right = Value::Dict(SharedMut::new(HashMap::new()));
        heap.adopt(&left);
        heap.adopt(&right);
        let Value::Dict(left_dict) = &left else {
            unreachable!()
        };
        let Value::Dict(right_dict) = &right else {
            unreachable!()
        };
        left_dict.write(|dict| dict.insert("right".into(), right.clone()));
        right_dict.write(|dict| dict.insert("left".into(), left.clone()));
        drop(left);
        drop(right);

        assert_eq!(heap.collect_cycles().collected, 2);
    }

    #[test]
    fn collects_self_referencing_object_and_assigns_stable_id() {
        let heap = ObjectHeap::default();
        let symbol = DefaultSymbol::try_from_usize(0).unwrap();
        let class = SharedMut::new(RuntimeClassDefinition::new(symbol, Default::default()));
        let object = Value::Object(SharedMut::new(ClassInstance::new(symbol, class)));
        heap.adopt(&object);
        let id = heap.object_id(&object).expect("managed ID");
        heap.adopt(&object);
        assert_eq!(heap.object_id(&object), Some(id));

        let Value::Object(instance) = &object else {
            unreachable!()
        };
        instance.write(|instance| {
            instance.field_values.insert(symbol, object.clone());
        });
        drop(object);

        assert_eq!(heap.collect_cycles().collected, 1);
    }

    #[test]
    fn builtin_results_are_registered_automatically() {
        let interner = goida_model::new_interner();
        let mut interpreter = Interpreter::new(interner.clone());
        BUILTINS.install(&mut interpreter).unwrap();
        let list_name = interner.write(|interner| interner.get_or_intern("list"));
        let builtin = interpreter.builtins.get(&list_name).unwrap().clone();
        let value = builtin(&interpreter, Vec::<CallArgValue>::new(), Default::default()).unwrap();
        let Value::List(list) = &value else {
            unreachable!()
        };
        list.write(|items| items.push(value.clone()));
        drop(value);

        assert_eq!(interpreter.collect_cycles().collected, 1);
    }

    #[test]
    fn collects_self_referencing_mutex_and_rwlock() {
        let heap = ObjectHeap::default();
        let mutex = Value::Mutex(RuntimeMutex::new(Value::Empty));
        let rwlock = Value::RwLock(RuntimeRwLock::new(Value::Empty));
        heap.adopt(&mutex);
        heap.adopt(&rwlock);
        let Value::Mutex(mutex_value) = &mutex else {
            unreachable!()
        };
        let Value::RwLock(rwlock_value) = &rwlock else {
            unreachable!()
        };
        *mutex_value.value.lock().unwrap() = mutex.clone();
        *rwlock_value.value.write().unwrap() = rwlock.clone();
        drop(mutex);
        drop(rwlock);

        assert_eq!(heap.collect_cycles().collected, 2);
    }

    #[test]
    fn interpreter_drop_collects_cycles_that_were_roots() {
        let interpreter = Interpreter::new(goida_model::new_interner());
        let list = SharedMut::new(Vec::new());
        let weak = list.downgrade();
        let value = interpreter.manage_value(Value::List(list.clone()));
        list.write(|items| items.push(value.clone()));
        interpreter.environment.write(|environment| {
            environment.define(DefaultSymbol::try_from_usize(0).unwrap(), value)
        });
        drop(list);

        drop(interpreter);

        assert!(weak.upgrade().is_none());
    }
}
