# Built-in entities

## Functions

| Canonical | Aliases |
|---|---|
| `print` | печать, print |
| `input` | ввод, input |
| `type` | тип, type |
| `is` | является, is |
| `number` | число, number |
| `string` | строка, string |
| `bool` | логический, bool |
| `float` | дробь, float |
| `list` | список, list |
| `array` | массив, array |
| `dict` | словарь, dict |
| `iterator` | итератор, iterator |
| `from_json` | из_json, from_json |
| `to_json` | в_json, to_json |
| `string_from_pointer` | строка_из_указателя, string_from_pointer |
| `regex` | регулярное_выражение, regex |

## Macros

| Canonical | Aliases |
|---|---|
| `format` | format, формат |

## Classes

### `String`

Aliases: Строка, String

| Method | Aliases | Static |
|---|---|---|
| `contains` | содержит, contains | no |
| `ends_with` | заканчивается_на, ends_with | no |
| `iterator` | итератор, iterator | no |
| `length` | длина, length | no |
| `lower` | нижний, lower | no |
| `replace` | заменить, replace | no |
| `split` | разделить, split | no |
| `starts_with` | начинается_с, starts_with | no |
| `trim` | обрезать, trim | no |
| `upper` | верхний, upper | no |

### `List`

Aliases: Список, List

| Method | Aliases | Static |
|---|---|---|
| `clear` | очистить, clear | no |
| `delete` | удалить, delete | no |
| `get` | получить, get | no |
| `iterator` | итератор, iterator | no |
| `join` | объединить, join | no |
| `length` | длина, length | no |
| `push` | добавить, push | no |
| `set` | задать, set | no |

### `Array`

Aliases: Массив, Array

| Method | Aliases | Static |
|---|---|---|
| `get` | получить, get | no |
| `iterator` | итератор, iterator | no |
| `join` | объединить, join | no |
| `length` | длина, length | no |

### `Dict`

Aliases: Словарь, Dict

| Method | Aliases | Static |
|---|---|---|
| `delete` | удалить, delete | no |
| `get` | получить, get | no |
| `has` | имеет, has | no |
| `iterator` | итератор, iterator | no |
| `keys` | ключи, keys | no |
| `length` | длина, length | no |
| `set` | задать, set | no |
| `values` | значения, values | no |

### `Iterator`

Aliases: Итератор, Iterator

| Method | Aliases | Static |
|---|---|---|
| `filter` | отфильтровать, filter | no |
| `map` | преобразовать, map | no |
| `reduce` | свернуть, reduce | no |
| `список` | список, list | no |

### `File`

Aliases: Файл, File

| Method | Aliases | Static |
|---|---|---|
| `append` | дописать, append | no |
| `delete` | удалить, delete | no |
| `exists` | существует, exists | no |
| `read` | прочитать, read | no |
| `write` | записать, write | no |

### `System`

Aliases: Система, System

| Method | Aliases | Static |
|---|---|---|
| `args` | аргументы, args | yes |
| `beep` | сигнал, beep | yes |
| `environment` | окружение, environment | yes |
| `exit` | выход, exit | yes |
| `panic` | паника, panic | yes |
| `platform` | платформа, platform | yes |
| `sleep` | сон, sleep | yes |
| `time` | время, time | yes |

### `Terminal`

Aliases: Терминал, Terminal

| Method | Aliases | Static |
|---|---|---|
| `clear` | очистить, clear | yes |
| `hide_cursor` | скрыть_курсор, hide_cursor | yes |
| `pause` | пауза, pause | yes |
| `position` | позиция, position | yes |
| `show_cursor` | показать_курсор, show_cursor | yes |
| `title` | заголовок, title | yes |

### `DateTime`

Aliases: ДатаВремя, DateTime

| Method | Aliases | Static |
|---|---|---|
| `add_days` | добавить_дней, add_days | no |
| `add_hours` | добавить_часов, add_hours | no |
| `add_minutes` | добавить_минут, add_minutes | no |
| `add_months` | добавить_месяцев, add_months | no |
| `add_seconds` | добавить_секунд, add_seconds | no |
| `add_years` | добавить_лет, add_years | no |
| `day` | день, day | no |
| `format` | формат, format | no |
| `hour` | час, hour | no |
| `minute` | минута, minute | no |
| `month` | месяц, month | no |
| `now` | сейчас, now | no |
| `second` | секунда, second | no |
| `sub_days` | вычесть_дней, sub_days | no |
| `sub_hours` | вычесть_часов, sub_hours | no |
| `sub_minutes` | вычесть_минут, sub_minutes | no |
| `sub_months` | вычесть_месяцев, sub_months | no |
| `sub_seconds` | вычесть_секунд, sub_seconds | no |
| `sub_years` | вычесть_лет, sub_years | no |
| `year` | год, year | no |

### `Regex`

Aliases: РегулярноеВыражение, Regex

| Method | Aliases | Static |
|---|---|---|
| `find` | найти, find | no |
| `find_all` | найти_все, find_all | no |
| `groups` | группы, groups | no |
| `groups_all` | группы_все, groups_all | no |
| `matches` | совпадает, matches | no |
| `pattern` | шаблон, pattern | no |
| `replace` | заменить, replace | no |
| `replace_all` | заменить_все, replace_all | no |
| `split` | разделить, split | no |

### `Thread`

Aliases: Поток, Thread

| Method | Aliases | Static |
|---|---|---|
| `create` | создать, create | yes |
| `wait` | ждать, wait | no |

### `Mutex`

Aliases: Мьютекс, Mutex

| Method | Aliases | Static |
|---|---|---|
| `lock` | блокировать, lock | no |
| `read` | прочитать, read | no |
| `unlock` | разблокировать, unlock | no |
| `write` | записать, write | no |

### `RwLock`

Aliases: БлокировкаЧтенияЗаписи, RwLock

| Method | Aliases | Static |
|---|---|---|
| `read` | прочитать, read | no |
| `read_lock` | читать_блокировать, read_lock | no |
| `read_unlock` | читать_разблокировать, read_unlock | no |
| `write` | записать, write | no |
| `write_lock` | писать_блокировать, write_lock | no |
| `write_unlock` | писать_разблокировать, write_unlock | no |

## Error Classes

| Class | Base |
|---|---|
| `Ошибка` | - |
| `ОшибкаПеременной` | Ошибка |
| `ОшибкаФункции` | Ошибка |
| `ОшибкаМетода` | Ошибка |
| `ОшибкаТипа` | Ошибка |
| `ОшибкаДеленияНаНоль` | Ошибка |
| `ОшибкаОперации` | Ошибка |
| `ОшибкаВводаВывода` | Ошибка |
| `ОшибкаИмпорта` | Ошибка |
| `Паника` | Ошибка |

## Types

| Aliases | Type |
|---|---|
| число, number | `Number` |
| строка, string | `Text` |
| логический, bool | `Boolean` |
| дробь, float | `Float` |
| указатель, pointer | `Pointer` |
| список, list | `List` |
| массив, array | `Array` |
| словарь, dict | `Dict` |
| пустота, void | `Unit` |
| неизвестно, any | `Any` |
| функция, function | `Function` |
| модуль, module | `Module` |
| ресурс, resource | `Resource` |
| класс, class | `Class` |

