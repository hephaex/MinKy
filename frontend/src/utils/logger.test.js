describe('logError', () => {
  let logError;
  let consoleSpy;

  beforeEach(() => {
    jest.resetModules();
    consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleSpy.mockRestore();
  });

  it('logs error in development mode', () => {
    process.env.NODE_ENV = 'development';
    logError = require('./logger').logError;
    const error = new Error('Test error');
    logError('TestContext', error);
    expect(consoleSpy).toHaveBeenCalled();
  });

  it('logs error with context', () => {
    process.env.NODE_ENV = 'development';
    logError = require('./logger').logError;
    const error = new Error('Test error');
    logError('MyComponent', error);
    expect(consoleSpy).toHaveBeenCalledWith('[MyComponent]', error, {});
  });

  it('logs error with metadata', () => {
    process.env.NODE_ENV = 'development';
    logError = require('./logger').logError;
    const error = new Error('Test error');
    const metadata = { userId: 123, action: 'delete' };
    logError('DeleteAction', error, metadata);
    expect(consoleSpy).toHaveBeenCalledWith('[DeleteAction]', error, metadata);
  });

  it('handles string errors', () => {
    process.env.NODE_ENV = 'development';
    logError = require('./logger').logError;
    logError('TestContext', 'String error message');
    expect(consoleSpy).toHaveBeenCalledWith('[TestContext]', 'String error message', {});
  });

  it('does not log in production mode', () => {
    process.env.NODE_ENV = 'production';
    logError = require('./logger').logError;
    const error = new Error('Test error');
    logError('TestContext', error);
    expect(consoleSpy).not.toHaveBeenCalled();
  });
});

describe('logWarning', () => {
  let logWarning;
  let consoleSpy;

  beforeEach(() => {
    jest.resetModules();
    consoleSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleSpy.mockRestore();
  });

  it('logs warning in development mode', () => {
    process.env.NODE_ENV = 'development';
    logWarning = require('./logger').logWarning;
    logWarning('TestContext', 'Warning message');
    expect(consoleSpy).toHaveBeenCalled();
  });

  it('logs warning with context', () => {
    process.env.NODE_ENV = 'development';
    logWarning = require('./logger').logWarning;
    logWarning('MyComponent', 'This is deprecated');
    expect(consoleSpy).toHaveBeenCalledWith('[MyComponent]', 'This is deprecated');
  });

  it('does not log in production mode', () => {
    process.env.NODE_ENV = 'production';
    logWarning = require('./logger').logWarning;
    logWarning('TestContext', 'Warning message');
    expect(consoleSpy).not.toHaveBeenCalled();
  });

  it('handles empty message', () => {
    process.env.NODE_ENV = 'development';
    logWarning = require('./logger').logWarning;
    logWarning('Context', '');
    expect(consoleSpy).toHaveBeenCalledWith('[Context]', '');
  });
});

describe('logInfo', () => {
  let logInfo;
  let consoleSpy;

  beforeEach(() => {
    jest.resetModules();
    consoleSpy = jest.spyOn(console, 'info').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleSpy.mockRestore();
  });

  it('logs info in development mode', () => {
    process.env.NODE_ENV = 'development';
    logInfo = require('./logger').logInfo;
    logInfo('TestContext', 'Info message');
    expect(consoleSpy).toHaveBeenCalled();
  });

  it('logs info with context', () => {
    process.env.NODE_ENV = 'development';
    logInfo = require('./logger').logInfo;
    logInfo('DataFetch', 'User data loaded successfully');
    expect(consoleSpy).toHaveBeenCalledWith('[DataFetch]', 'User data loaded successfully');
  });

  it('does not log in production mode', () => {
    process.env.NODE_ENV = 'production';
    logInfo = require('./logger').logInfo;
    logInfo('TestContext', 'Info message');
    expect(consoleSpy).not.toHaveBeenCalled();
  });

  it('handles special characters', () => {
    process.env.NODE_ENV = 'development';
    logInfo = require('./logger').logInfo;
    logInfo('Component', 'Message with émojis 🎉 and spéciål chars');
    expect(consoleSpy).toHaveBeenCalledWith('[Component]', 'Message with émojis 🎉 and spéciål chars');
  });
});
