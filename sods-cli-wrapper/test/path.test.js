const path = require('path');
const os = require('os');

describe('npm wrapper path handling', () => {
    test('path.resolve handles current directory correctly', () => {
        const currentDir = process.cwd();
        const workspacePath = path.resolve(currentDir);

        // On Windows, this should have a drive letter and backslashes
        // On Unix, it should start with /
        if (os.platform() === 'win32') {
            expect(workspacePath).toMatch(/^[a-zA-Z]:\\/);
        } else {
            expect(workspacePath).toMatch(/^\//);
        }
    });

    test('path.join and resolve consistency', () => {
        const base = os.tmpdir();
        const sub = 'sods-test';
        const combined = path.resolve(base, sub);

        expect(combined).toContain(sub);
        expect(combined).toContain(base);
    });
});
