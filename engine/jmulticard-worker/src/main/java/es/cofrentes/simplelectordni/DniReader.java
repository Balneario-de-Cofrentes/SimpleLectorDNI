package es.cofrentes.simplelectordni;

@FunctionalInterface
interface DniReader {
    DniReadResult read(int readerIndex) throws Exception;
}
