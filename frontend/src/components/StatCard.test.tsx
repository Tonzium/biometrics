import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { StatCard } from './StatCard'

describe('StatCard', () => {
  it('renders label, value and hint', () => {
    render(<StatCard label="Unipisteet" value="82" hint="viime yö" accent="blue" />)
    expect(screen.getByText('Unipisteet')).toBeInTheDocument()
    expect(screen.getByText('82')).toBeInTheDocument()
    expect(screen.getByText('viime yö')).toBeInTheDocument()
  })

  it('omits the hint when not given', () => {
    const { container } = render(<StatCard label="Askeleet" value="8 823" />)
    expect(container.querySelector('.stat-hint')).toBeNull()
  })
})
