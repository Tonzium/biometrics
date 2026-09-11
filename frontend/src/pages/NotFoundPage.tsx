import { Link } from 'react-router'

export function NotFoundPage() {
  return (
    <section className="page page-narrow">
      <h1>Sivua ei löydy</h1>
      <p className="muted">Osoite ei vastaa mitään sivua.</p>
      <Link to="/" className="btn btn-primary">
        Takaisin yleiskuvaan
      </Link>
    </section>
  )
}
