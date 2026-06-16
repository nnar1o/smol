#!/usr/bin/env bash
# Simulated Maven build + test output for integration testing.
# Mixes compilation errors, test runner lines, and test summaries
# to verify that the interactive parser correctly separates
# build errors from test failures.

cat <<'EOF'
[INFO] Scanning for projects...
[INFO]
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ my-app ---
[INFO] Compiling 42 source files to /target/classes
[ERROR] /src/main/java/com/example/App.java:15: error: cannot find symbol
  symbol:   variable missingField
  location: class App
[ERROR] /src/main/java/com/example/App.java:22: error: incompatible types
[WARNING] /src/main/java/com/example/App.java:10: unchecked cast
[WARNING] /src/main/java/com/example/Utils.java:5: rawtypes warning
[INFO]
[INFO] --- maven-surefire-plugin:3.0.0:test (default-test) @ my-app ---
[INFO]
[INFO] Running com.example.SimpleTest
Tests run: 5, Failures: 0, Errors: 0, Skipped: 0, Time elapsed: 0.012 s
[INFO] Running com.example.FastDateParserTest
Tests run: 15, Failures: 2, Errors: 1, Skipped: 3, Time elapsed: 0.234 s <<< FAILURE!
Tests in error:
  FastDateParserTest.testFormatError:66 » NullPointer
Failed tests:
  FastDateParserTest.testParseError:42 expected:<2024-01-01> but was:<2024-01-02>
  FastDateParserTest.testLenient:99 expected:<true> but was:<false>
[INFO] Running com.example.EdgeCaseTest
Tests run: 8, Failures: 0, Errors: 0, Skipped: 1, Time elapsed: 0.089 s
[INFO]
[INFO] Results:
[INFO] Tests run: 28, Failures: 2, Errors: 1, Skipped: 4
[INFO]
[INFO] ------------------------------------------------------------------------
[INFO] BUILD FAILURE
[INFO] ------------------------------------------------------------------------
[INFO] Total time:  12.345 s
EOF
exit 1
