import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

// Ilman `globals: true` RTL ei siivoa DOM:ia automaattisesti testien välissä.
afterEach(() => cleanup())
