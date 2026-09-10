import marca from "../assets/marca.png";

/** La marca de la aplicación, la misma que el icono del sistema. */
export default function Marca({ px = 34 }: { px?: number }) {
  return <img className="marca" src={marca} width={px} height={px} alt="IureDav" />;
}
