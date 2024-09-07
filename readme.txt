Конвертація в hex
cargo-objcopy target/thumbv7em-none-eabihf/release/two --release -- -O ihex two.hex

Дизассемблер
cargo objdump --bin two --release -- --disassemble --no-show-raw-insn --print-imm-hex

Запуск OpenOCD
openocd -f interface/stlink.cfg -f target/stm32f4x.cfg

Взнати розмір програми
cargo size --bin fan_controller --release -- -A
.text містить інструкції до програми
.rodata містить постійні значення, такі як рядки
.data містить статично виділені змінні, початкові значення яких не дорівнюють нулю
.bss також містить статично виділені змінні, початкові значення яких дорівнюють нулю
.vector_table це нестандартний розділ, який ми використовуємо для зберігання таблиці векторів (переривань).
.ARM.attributes і .debug_* розділи містять метадані та не будуть завантажені в ціль під час прошивки двійкового файлу.

Прошивка 
cargo flash --chip stm32f401ccu6 --release 

Компіляція і прошивка
cargo run --release
Або
cargo embed --release
