// Stable TCC responsible application for rebuildable native acceptance tests.
#import <Foundation/Foundation.h>
int main(void) {
    @autoreleasepool {
        NSString *directory = NSBundle.mainBundle.bundlePath.stringByDeletingLastPathComponent;
        NSTask *test = [NSTask new];
        test.executableURL = [NSURL fileURLWithPath:[directory stringByAppendingPathComponent:@"window-tests"]];
        NSArray *arguments = NSProcessInfo.processInfo.arguments;
        test.arguments = [arguments subarrayWithRange:NSMakeRange(1, arguments.count - 1)];
        NSError *error = nil;
        if (![test launchAndReturnError:&error]) {
            fprintf(stderr, "%s\n", error.localizedDescription.UTF8String);
            return 1;
        }
        [test waitUntilExit];
        return test.terminationStatus;
    }
}
