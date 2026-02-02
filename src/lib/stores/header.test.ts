import { describe, it, expect } from 'vitest';
import { headerStore } from './header.svelte';

describe('headerStore', () => {
    it('should initialize with default title', () => {
        expect(headerStore.title).toBe('Dashboard');
    });

    it('should update title', () => {
        headerStore.setTitle('Test Page');
        expect(headerStore.title).toBe('Test Page');
    });
});
