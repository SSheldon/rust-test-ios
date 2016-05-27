#import <XCTest/XCTest.h>

size_t tests_count();
const char *test_name(size_t i, size_t *len);
int run_test(size_t i);

@interface RustTests : XCTestCase
@end

@implementation RustTests

- (void)test {
  for (size_t i = 0; i < tests_count(); i++) {
    size_t name_len;
    const char *c_name = test_name(i, &name_len);
    NSString *name = [[NSString alloc] initWithBytes:c_name length:name_len encoding:NSUTF8StringEncoding];

    NSLog(@"Running test: %@", name);
    int result = run_test(i);
    XCTAssert(result != 0, @"Failed %@", name);
  }
}

@end
