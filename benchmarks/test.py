def arithmetic(rounds, limit):
    checksum = 0
    for _ in range(rounds):
        total = 0
        for i in range(limit):
            total += i * 3 + 1
        checksum += total
    return checksum

# Вызов функции
print(arithmetic(8, 50000))  # Выведет: 29999800000
