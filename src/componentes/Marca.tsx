/** La marca del icono, en linea, para que no dependa de cargar un fichero. */
export default function Marca({ px = 34 }: { px?: number }) {
  return (
    <svg width={px} height={px} viewBox="0 0 100 100" aria-label="IureDav" role="img">
      <rect width="100" height="100" rx="22" fill="var(--azul)" />
      <g fill="#fff">
        <circle cx="35.5" cy="37.5" r="11.5" />
        <circle cx="50" cy="33.5" r="15.5" />
        <circle cx="64.5" cy="38.5" r="10.5" />
        <rect x="35.5" y="39" width="29" height="10" />
        <rect x="20" y="60.5" width="60" height="13" rx="3.2" />
        <rect x="28.5" y="77.5" width="43" height="7" rx="2.6" opacity="0.57" />
      </g>
      <circle cx="71.5" cy="67" r="3" fill="var(--agua)" />
    </svg>
  );
}
