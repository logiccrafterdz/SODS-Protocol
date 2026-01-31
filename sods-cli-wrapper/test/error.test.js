const { spawn } = require('child_process');
const os = require('os');
const path = require('path');

// We mock spawn to simulate Docker not being installed
jest.mock('child_process', () => ({
    spawn: jest.fn()
}));

describe('npm wrapper error handling', () => {
    let consoleSpy;
    let exitSpy;

    beforeEach(() => {
        consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => { });
        exitSpy = jest.spyOn(process, 'exit').mockImplementation(() => { });
        jest.resetModules();
    });

    afterEach(() => {
        consoleSpy.mockRestore();
        exitSpy.mockRestore();
    });

    test('docker not found error displays platform-specific instructions', (done) => {
        const { spawn } = require('child_process');
        const EventEmitter = require('events');
        const mockChild = new EventEmitter();

        spawn.mockReturnValue(mockChild);

        // Required to trigger the actual script logic in a testable way
        // We'll require the script but we need to prevent it from running automatically or mock the logic
        // Since sods.js runs immediately on require, we test the logic directly or refactor to export

        const platform = os.platform();
        let expectedUrl = '';
        if (platform === 'win32') expectedUrl = 'desktop/install/windows-install/';
        else if (platform === 'darwin') expectedUrl = 'desktop/install/mac-install/';
        else expectedUrl = 'engine/install/';

        // Simulate ENOENT
        setTimeout(() => {
            mockChild.emit('error', { code: 'ENOENT' });

            expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Docker is not installed'));
            expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining(expectedUrl));
            expect(exitSpy).toHaveBeenCalledWith(1);
            done();
        }, 10);

        // Trigger the logic (this is tricky because sods.js is a script)
        // For testing purposes, we define the handleError logic in the test or refactor sods.js
        // Here we'll just verify the logic we implemented in sods.js indirectly
        require('../bin/sods.js');
    });
});
