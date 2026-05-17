# Emboar OS - EmShell Language Specification

## 1. Overview

EmShell is a secure, typed shell language combining Bash familiarity with strict type safety and cryptographic security. It eliminates entire categories of vulnerabilities through compile-time type checking.

---

## 2. Type System

### 2.1 Primitive Types

```
int          - 64-bit signed integer (-2^63 to 2^63-1)
uint         - 64-bit unsigned integer (0 to 2^64-1)
float        - IEEE 754 double precision
bool         - true or false
string       - UTF-8 encoded text (immutable)
path         - Filesystem path with validation
bytes        - Raw binary data
```

### 2.2 Composite Types

```emshell
# Arrays (homogeneous)
array<int> numbers = [1, 2, 3, 4, 5]
array<string> names = ["alice", "bob", "charlie"]
array<path> dirs = ["/tmp", "/var", "/home"]

# Maps (key-value pairs)
map<string, string> config = {
    "hostname": "emboar-1",
    "timezone": "UTC"
}

# Tuples (fixed heterogeneous)
(string, int, bool) person = ("alice", 35, true)

# Optional types (nullable)
?string maybe_token = null
?int session_timeout = 15
```

### 2.3 Type Inference

```emshell
# Explicit type
int x = 42

# Inferred from literal
y = 42              # Inferred as int

# Inferred from expression
z = x + 10          # Inferred as int

# No implicit coercion across boundaries
s = "hello"         # string
n = 42              # int
result = s + n      # ERROR: Cannot add string + int
```

---

## 3. Variables & Scope

### 3.1 Declaration & Assignment

```emshell
# In function scope
function example() {
    int count = 10
    string name = "system"
    path config_path = "/etc/emboar/config"
    
    # Variables live until function exit
}

# Shell scope (global)
SHELL_VAR = "visible everywhere"

# Immutable
const DATABASE_URL = "localhost:5432"
DATABASE_URL = "remote"    # ERROR: Cannot reassign const
```

### 3.2 Scope Rules

```emshell
int global_var = 100

function outer() {
    int outer_var = 200
    
    function inner() {
        int inner_var = 300
        echo global_var      # ✓ 100 (accessible)
        echo outer_var       # ✓ 200 (accessible)
        echo inner_var       # ✓ 300 (own scope)
    }
    
    inner()
    echo inner_var           # ✗ ERROR: Not in scope
}
```

---

## 4. Operators

### 4.1 Arithmetic

```emshell
x = 10
y = 3

addition = x + y            # 13
subtraction = x - y         # 7
multiplication = x * y      # 30
division = x / y            # 3 (integer division)
modulo = x % y              # 1
exponent = x ** 2           # 100
```

### 4.2 Comparison

```emshell
# Numeric
10 > 5                  # true
10 >= 10                # true
5 < 10                  # true
5 <= 5                  # true
10 == 10                # true
10 != 5                 # true

# String
"hello" == "hello"      # true
"a" < "b"               # true (lexicographic)
```

### 4.3 Logical

```emshell
true && false           # false (AND)
true || false           # true  (OR)
!true                    # false (NOT)

# Short-circuit evaluation
if true || expensive_check() {
    # expensive_check() is never called
}
```

### 4.4 String Operations

```emshell
s1 = "hello"
s2 = "world"

concatenation = s1 + " " + s2              # "hello world"
substring = s1[0:2]                        # "he"
length = len(s1)                            # 5
upper = s1.to_upper()                      # "HELLO"
contains = "world" in s2                   # true
split = "a,b,c".split(",")                 # ["a", "b", "c"]
```

### 4.5 Path Operations

```emshell
p1 = /home/alice
p2 = /documents

# Path joining
combined = p1 / p2                  # /home/alice/documents

# Path components
basename = p1.basename()            # "alice"
dirname = p1.dirname()              # "/home"
exists = p1.exists()                # true/false
is_file = p1.is_file()              # true/false
is_dir = p2.is_dir()                # true/false
```

---

## 5. Control Flow

### 5.1 Conditionals

```emshell
# if-else-if
if [[ $count > 0 ]]; then
    echo "Positive"
elif [[ $count < 0 ]]; then
    echo "Negative"
else
    echo "Zero"
fi

# Inline if-else (ternary)
status = count > 0 ? "positive" : "negative"

# switch-case
switch command {
    case "start":
        service_start()
    case "stop":
        service_stop()
    default:
        echo "Unknown command"
}
```

### 5.2 Loops

```emshell
# for loop over array
for file in ["/tmp/a", "/tmp/b", "/tmp/c"] {
    echo "Processing: $file"
}

# for loop with range
for i in 1..10 {
    echo "Count: $i"
}

# while loop
count = 0
while [[ $count < 5 ]] {
    echo $count
    count = count + 1
}

# break and continue
for i in 1..10 {
    if [[ i == 3 ]]; then
        continue
    fi
    if [[ i == 7 ]]; then
        break
    fi
    echo $i
}
```

### 5.3 Pattern Matching

```emshell
# Match on value
result = match status {
    "success" => "Operation completed",
    "error" => "Operation failed",
    "pending" => "In progress",
    _ => "Unknown status"
}

# Match with conditions
match (user_id, permission) {
    (_, "admin") => grant_access(),
    (1001, "write") => grant_partial_access(),
    _ => deny_access()
}
```

---

## 6. Functions

### 6.1 Declaration & Calling

```emshell
# Typed function signature
function add(int a, int b) -> int {
    return a + b
}

# Function with multiple returns
function divide(int numerator, int denominator) -> (int, int) {
    quotient = numerator / denominator
    remainder = numerator % denominator
    return (quotient, remainder)
}

# No return value (void)
function log_message(string msg) {
    echo "[$(date)] $msg"
}

# Calling functions
result = add(10, 20)                    # 30
(q, r) = divide(10, 3)                  # (3, 1)
log_message("Application started")
```

### 6.2 Default Arguments

```emshell
function create_backup(path source, path dest = /backup/) {
    tar --create --file="$dest/backup.tar" "$source"
}

create_backup(/data)                  # Uses default dest
create_backup(/data, /mnt/backup)    # Override default
```

### 6.3 Variable Arguments

```emshell
function echo_all(string ...args) {
    for msg in args {
        echo "Message: $msg"
    }
}

echo_all("hello", "world", "test")
```

### 6.4 Higher-Order Functions

```emshell
# Function taking another function as parameter
function map(array<int> arr, function<int, int> f) -> array<int> {
    result = []
    for item in arr {
        result.push(f(item))
    }
    return result
}

double = function(int x) -> int { return x * 2 }
numbers = [1, 2, 3, 4, 5]
doubled = map(numbers, double)  # [2, 4, 6, 8, 10]
```

### 6.5 Closures

```emshell
function make_adder(int n) -> function<int, int> {
    return function(int x) -> int { return x + n }
}

add5 = make_adder(5)
result = add5(10)               # 15
```

---

## 7. Pipes & Redirection

### 7.1 Basic Piping

```emshell
# Pipe output to next command
ps aux | grep "sshd" | wc -l

# Multiple pipes
cat /var/log/app.log | grep "ERROR" | head -20 | sort

# Pipe as array
files = (ls -1 | grep ".txt")   # files now contains array of .txt files
```

### 7.2 Output Redirection

```emshell
# Redirect stdout
echo "Hello" > /tmp/output.txt

# Append to file
echo "World" >> /tmp/output.txt

# Redirect stderr
command_with_error 2> /tmp/error.log

# Redirect both
command_mixed 2>&1 > /tmp/combined.log

# Redirect to different files
command 1> stdout.log 2> stderr.log
```

### 7.3 Input Redirection

```emshell
# Read from file
sort < /tmp/unsorted.txt > /tmp/sorted.txt

# Here document
sql = cat << EOF
    SELECT * FROM users WHERE id = 1;
    UPDATE users SET active = 1;
EOF
```

### 7.4 Process Substitution

```emshell
# Compare two command outputs
diff <(ls /dir1) <(ls /dir2)

# Multiple inputs
sort_combined <(sort /file1) + <(sort /file2)
```

---

## 8. String Interpolation

### 8.1 Variable Expansion

```emshell
name = "alice"
count = 42

# Simple interpolation
message = "Hello, $name!"                    # "Hello, alice!"
message = "Count: $count"                   # "Count: 42"

# Expression interpolation
result = "Sum: ${x + y}"                    # "Sum: 25"
```

### 8.2 Command Substitution

```emshell
# Using $()
current_time = "Current time: $(date)"

# Using backticks (legacy, discouraged)
uptime = `uptime`

# Nested substitution
nested = "Result: $(echo $(ls /tmp))"
```

---

## 9. Error Handling

### 9.1 Exit Codes

```emshell
# Check exit code of last command
last_status = $?

# Explicit error handling
result = encrypt /file.txt --key-vault
if [[ $? -ne 0 ]]; then
    echo "Encryption failed!" --color=red
    exit 1
fi

# Short-circuit on error
command1 && command2 && command3

# Fallback on error
command1 || echo "command1 failed, trying alternative"
```

### 9.2 Try-Catch

```emshell
try {
    data = file_read("/secret/data.txt")
    process(data)
} catch error {
    echo "Error: $error" --color=red
    audit_log("Failed to read secret data")
}
```

### 9.3 Panic & Recover

```emshell
# Intentional panic
if [[ $user != "admin" ]]; then
    panic("Unauthorized access attempt")
fi

# Recover from panic
if !recover {
    echo "Panic recovery failed"
    exit 1
}
```

---

## 10. ANSI Colors & Formatting

### 10.1 Color Output

```emshell
# Using echo with color
echo "Success!" --color=green --bold
echo "Error!" --color=red --bold
echo "Warning" --color=yellow
echo "Info" --color=cyan
echo "Debug" --color=dark-gray --dimmed

# Using format strings
formatted = "%{green}✓%{reset} Operation successful"
echo formatted

# 256-color support
echo "Custom color" --color=208     # Orange
```

### 10.2 Formatting Codes

```emshell
# Text attributes
--bold, --dim, --italic, --underline, --blink

# Foreground colors
--fg=black, --fg=red, --fg=green, --fg=yellow
--fg=blue, --fg=magenta, --fg=cyan, --fg=white
--fg=256-color-code (0-255)

# Background colors
--bg=red, --bg=green (256 colors available)

# Reset
--reset
```

### 10.3 Progress Indicators

```emshell
# Progress bar
progress 50%
progress "50/100"
progress --bar="████░░░░░░" 50%

# Spinner
spinner --type=dots "Processing..."
```

---

## 11. Built-in Functions

### 11.1 String Functions

```emshell
len("hello")              # 5
upper("hello")           # "HELLO"
lower("HELLO")           # "hello"
trim("  hello  ")        # "hello"
split("a,b,c", ",")      # ["a", "b", "c"]
join(["a", "b"], ",")    # "a,b"
replace("hello", "l", "r")  # "herro"
contains("hello", "ell") # true
startswith("hello", "he") # true
endswith("hello", "lo")  # true
substring("hello", 1, 3) # "ell"
```

### 11.2 Math Functions

```emshell
abs(-42)                # 42
min(1, 2, 3)            # 1
max(1, 2, 3)            # 3
sqrt(16)                # 4
pow(2, 3)               # 8
round(3.7)              # 4
floor(3.7)              # 3
ceil(3.2)               # 4
random()                # 0.123456...
random_range(1, 100)    # random int in [1, 100]
```

### 11.3 Array Functions

```emshell
arr = [1, 2, 3, 4, 5]

len(arr)                # 5
arr.push(6)             # arr now [1, 2, 3, 4, 5, 6]
arr.pop()               # Returns 6, arr is [1, 2, 3, 4, 5]
arr.shift()             # Returns 1, arr is [2, 3, 4, 5]
arr.reverse()           # [5, 4, 3, 2]
arr.sort()              # [1, 2, 3, 4, 5]
arr.filter(x > 2)       # [3, 4, 5]
arr.map(x * 2)          # [2, 4, 6, 8, 10]
arr.join(",")           # "1,2,3,4,5"
```

### 11.4 Cryptographic Functions

```emshell
# Hashing
sha512("password")                  # 512-bit hex digest
sha256("data")                      # 256-bit hex digest

# Encryption
encrypt("plaintext", key)           # Returns ciphertext
decrypt("ciphertext", key)          # Returns plaintext

# Random generation
random_bytes(32)                    # 32 cryptographically secure bytes
random_string(16)                   # Random alphanumeric string
uuid()                              # UUID v4

# Key management
keygen("rsa", 4096)                 # Generate RSA-4096 key
sign("data", private_key)           # Digital signature
verify("data", signature, pub_key)  # Verify signature
```

---

## 12. Command Structure

### 12.1 Command Format

```emshell
# Command with arguments
ls /home/alice

# Command with flags
ls -la /home/alice

# Command with options (key=value)
encrypt file.txt --algorithm=aes-256 --key-vault

# Combining flags and options
tar -czf --verbose archive.tar.gz /data
```

### 12.2 Argument Parsing

```emshell
# Positional arguments
copy <source> <destination>

# Optional arguments (default values)
backup [--destination=/backup] [--compress=true]

# Variadic arguments
echo [messages...]

# Mixed
openssl [--verbose] <command> [args...]
```

---

## 13. Comments

```emshell
# Single-line comment

/* Multi-line
   comment spans
   many lines */

function process() {
    # TODO: Implement error handling
    x = 42  # Inline comment
}
```

---

## 14. Grammar (BNF)

```
program     := (declaration | statement)*

declaration := function_decl | const_decl
function_decl := "function" IDENT "(" params ")" ["->" type] "{" statement* "}"
const_decl  := "const" IDENT "=" expression

statement   := assignment | if_stmt | while_stmt | for_stmt
             | expression_stmt | return_stmt | break | continue

assignment  := IDENT "=" expression

if_stmt     := "if" "[[" expression "]]" ";" "then" 
               statement* ["elif" ...] ["else" statement*] "fi"

while_stmt  := "while" "[[" expression "]]" "{" statement* "}"

for_stmt    := "for" IDENT "in" expression "{" statement* "}"

return_stmt := "return" [expression]

expression  := logical_or

logical_or  := logical_and ("||" logical_and)*
logical_and := comparison ("&&" comparison)*
comparison  := addition ((">" | "<" | ">=" | "<=" | "==" | "!=") addition)*
addition    := multiplication (("+" | "-") multiplication)*
multiplication := power (("*" | "/" | "%") power)*
power       := unary ("**" unary)*
unary       := ("!" | "-") unary | postfix
postfix     := primary ("[" expression "]")*
primary     := IDENT | NUMBER | STRING | PATH | "(" expression ")"
             | pipe_expr
```

---

*Emboar OS EmShell Language Specification v1.0*
