// Disposable external AX provider for the explicitly run Rust native probe.
#import <AppKit/AppKit.h>
int main(void) {
    @autoreleasepool {
        NSApplication *app = NSApplication.sharedApplication;
        [app setActivationPolicy:NSApplicationActivationPolicyAccessory];
        CGFloat mainHeight = NSScreen.screens.firstObject.frame.size.height;
        for (NSScreen *screen in NSScreen.screens) {
            NSRect frame = screen.frame, work = screen.visibleFrame;
            printf("%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f;",
                   frame.origin.x, mainHeight - NSMaxY(frame), frame.size.width, frame.size.height,
                   work.origin.x, mainHeight - NSMaxY(work), work.size.width, work.size.height, screen.backingScaleFactor);
        }
        printf("\n"); fflush(stdout);
        NSMutableArray<NSWindow *> *windows = [NSMutableArray new];
        for (NSInteger index = 0; index < 3; index++) {
            NSWindow *window = [[NSWindow alloc]
                initWithContentRect:NSMakeRect(160, 200, 480, 320)
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable
                backing:NSBackingStoreBuffered defer:NO];
            window.title = [NSString stringWithFormat:@"KeySteer parity probe %ld", (long)index];
            window.minSize = NSMakeSize(240, 160);
            window.releasedWhenClosed = NO;
            [window makeKeyAndOrderFront:nil];
            [windows addObject:window];
        }
        [app activateIgnoringOtherApps:YES];
        NSFileHandle.fileHandleWithStandardInput.readabilityHandler = ^(NSFileHandle *input) {
            NSData *command = input.availableData;
            if (!command.length) { input.readabilityHandler = nil; return; }
            dispatch_async(dispatch_get_main_queue(), ^{
                NSWindow *window = windows.firstObject;
                NSPoint origin = window.frame.origin;
                origin.x = origin.x >= 360 ? 160 : origin.x + 1;
                [window setFrameOrigin:origin];
            });
        };

        // Also clean up if the parent test crashes.
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 180 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
            [app terminate:nil];
        });
        [app run];
    }
    return 0;
}
