import { Node } from './Node';
import { Runner } from './runner';
import fc from 'fast-check';

// Mock the static run method of the Runner class
jest.mock('./runner', () => {
  return {
    Runner: {
      run: jest.fn(),
    },
  };
});

describe('Node class', () => {
  beforeEach(() => {
    // Clear all mocks before each test
    jest.clearAllMocks();
  });

  describe('create method', () => {
    test('should always return a Node instance with the correct name', async () => {
      Runner.run.mockResolvedValue({ code: 0, stdout: '', stderr: '' });

      await fc.assert(
        fc.asyncProperty(fc.string(), async (name) => {
          const node = await Node.create(name);
          expect(node).toBeInstanceOf(Node);
          expect(node.name).toBe(name);
        })
      );
    });

    test('should throw an error when the runner fails', async () => {
      Runner.run.mockRejectedValue(new Error('Command failed'));

      await fc.assert(
        fc.asyncProperty(fc.string(), async (name) => {
          await expect(Node.create(name)).rejects.toThrow('Failed to create node');
        })
      );
    });
  });

  describe('show method', () => {
    test('should return true when the node show command is successful', async () => {
      Runner.run.mockResolvedValue({ code: 0, stdout: '', stderr: '' });

      await fc.assert(
        fc.asyncProperty(fc.string(), async (name) => {
          const node = new Node(name);
          const result = await node.show();
          expect(result).toBe(true);
        })
      );
    });

    test('should throw an error when the node show command fails', async () => {
      Runner.run.mockResolvedValue({ code: 1, stdout: '', stderr: 'Error' });

      await fc.assert(
        fc.asyncProperty(fc.string(), async (name) => {
          const node = new Node(name);
          await expect(node.show()).rejects.toThrow('Failed to get status');
        })
      );
    });
  });

  describe('delete method', () => {
    test('should return true when the node delete command is successful', async () => {
      Runner.run.mockResolvedValue({ code: 0, stdout: '', stderr: '' });

      await fc.assert(
        fc.asyncProperty(fc.string(), async (name) => {
          const node = new Node(name);
          const result = await node.delete();
          expect(result).toBe(true);
        })
      );
    });

    test('should throw an error when the node delete command fails', async () => {
      Runner.run.mockResolvedValue({ code: 1, stdout: '', stderr: 'Error' });

      await fc.assert(
        fc.asyncProperty(fc.string(), async (name) => {
          const node = new Node(name);
          await expect(node.delete()).rejects.toThrow('Failed to delete node');
        })
      );
    });
  });
});
