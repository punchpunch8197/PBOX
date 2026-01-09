use pbox::num;

fn main() {
    println!("=== Precision-Box Demo ===\n");

    // 1. IEEE 754 문제 해결
    println!("1. IEEE 754 Problem Solved:");
    println!("   IEEE 754: 0.1 + 0.2 = 0.30000000000000004");
    println!("   pbox:     0.1 + 0.2 = {}\n", num("0.1") + num("0.2"));

    // 2. 순환 소수
    println!("2. Repeating Decimals:");
    let result = num("0.(3)") + num("0.(3)") + num("0.(3)");
    println!("   1/3 + 1/3 + 1/3 = {}\n", result);

    // 3. 거듭제곱
    println!("3. Power:");
    println!("   1.1^3 = {}", num("1.1").pow(3));
    println!("   1.1^10 = {}\n", num("1.1").pow(10));

    // 4. 사칙연산
    println!("4. Arithmetic:");
    println!("   123.456 + 78.9 = {}", num("123.456") + num("78.9"));
    println!("   123.456 - 78.9 = {}", num("123.456") - num("78.9"));
    println!("   123.456 * 78.9 = {}", num("123.456") * num("78.9"));
}
