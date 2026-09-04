package es.cofrentes.simplelectordni;

@FunctionalInterface
interface DniReader {
    DniReadResult read(String readerName) throws Exception;
}
