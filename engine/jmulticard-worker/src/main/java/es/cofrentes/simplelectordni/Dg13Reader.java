package es.cofrentes.simplelectordni;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;

import es.gob.jmulticard.DigestAlgorithm;
import es.gob.jmulticard.HexUtils;
import es.gob.jmulticard.asn1.icao.DataGroupHash;
import es.gob.jmulticard.asn1.icao.LdsSecurityObject;
import es.gob.jmulticard.asn1.icao.OptionalDetails;
import es.gob.jmulticard.asn1.icao.Sod;
import es.gob.jmulticard.card.Location;
import es.gob.jmulticard.card.dnie.Dnie;
import es.gob.jmulticard.card.dnie.Dnie3;
import es.gob.jmulticard.card.dnie.DnieFactory;
import es.gob.jmulticard.connection.ApduConnectionProtocol;
import es.gob.jmulticard.crypto.BcCryptoHelper;
import es.gob.jmulticard.jse.smartcardio.SmartcardIoConnection;

final class Dg13Reader implements DniReader {

    @Override
    public DniReadResult read(final String readerName) throws DniReadException {
        final SmartcardIoConnection connection = new SmartcardIoConnection();
        try {
            return readConnected(connection, readerName);
        }
        catch (final DniReadException e) {
            throw e;
        }
        catch (final Exception e) {
            throw new DniReadException("CARD_READ_FAILED", "could not read DNIe", true, e);
        }
        finally {
            closeQuietly(connection);
        }
    }

    private static DniReadResult readConnected(
        final SmartcardIoConnection connection,
        final String readerName
    ) throws Exception {
        connection.setProtocol(ApduConnectionProtocol.T0);
        final int readerIndex = findReader(connection, readerName);
        connection.setTerminal(readerIndex);
        final BcCryptoHelper crypto = new BcCryptoHelper();
        final Dnie card = DnieFactory.getDnie(connection, null, crypto, null, false);
        if (!(card instanceof Dnie3 dnie)) {
            throw new DniReadException(
                "UNSUPPORTED_CARD",
                "inserted card is not a supported DNIe",
                false,
                null
            );
        }
        return readDg13(dnie, crypto);
    }

    private static int findReader(
        final SmartcardIoConnection connection,
        final String readerName
    ) throws Exception {
        for (final long terminal : connection.getTerminals(false)) {
            final int index = Math.toIntExact(terminal);
            if (readerName.equals(connection.getTerminalInfo(index))) {
                return index;
            }
        }
        throw new DniReadException(
            "READER_NOT_FOUND",
            "configured reader is unavailable",
            true,
            null
        );
    }

    private static DniReadResult readDg13(
        final Dnie3 dnie,
        final BcCryptoHelper crypto
    ) throws Exception {
        dnie.openSecureChannelIfNotAlreadyOpened(false);
        final OptionalDetails details = dnie.getDg13();
        final DocumentData document = mapDocument(dnie, details);
        final IntegrityResult integrity = verifyDg13(dnie, details.getBytes(), crypto);
        return new DniReadResult(document, integrity);
    }

    private static DocumentData mapDocument(
        final Dnie3 dnie,
        final OptionalDetails details
    ) {
        final Dg13TextFields raw = Dg13TextFields.from(details.getBytes());
        final DocumentData value = new DocumentData();
        value.nombre = clean(details.getName());
        value.primer_apellido = clean(details.getFirstSurname());
        value.segundo_apellido = clean(details.getSecondSurname());
        value.apellidos = join(value.primer_apellido, value.segundo_apellido);
        value.dni_formateado = clean(details.getIdNumber());
        value.dni = normalizeDocumentNumber(value.dni_formateado);
        value.fecha_nacimiento = raw.isoDateAt(5);
        value.nacionalidad = clean(details.getNationality());
        value.fecha_caducidad = raw.isoDateAt(7);
        mapAdditionalDetails(value, details);
        value.version_dnie = optionalText(() -> readDnieVersion(dnie));
        value.serial_chip = optionalText(() -> HexUtils.hexify(dnie.getSerialNumber(), false));
        return value;
    }

    private static void mapAdditionalDetails(
        final DocumentData value,
        final OptionalDetails details
    ) {
        value.numero_soporte = clean(details.getSupportNumber());
        value.sexo = details.getSex() == null ? "" : details.getSex().toString();
        value.ciudad_nacimiento = clean(details.getBirthCity());
        value.provincia_nacimiento = clean(details.getBirthProvince());
        value.pais_nacimiento = clean(details.getBirthCountry());
        value.nombres_progenitores = clean(details.getParentsNames());
        value.direccion = clean(details.getAddress());
        value.localidad = clean(details.getCity());
        value.provincia = clean(details.getProvince());
        value.pais = clean(details.getCountry());
    }

    private static IntegrityResult verifyDg13(
        final Dnie3 dnie,
        final byte[] dg13Bytes,
        final BcCryptoHelper crypto
    ) throws Exception {
        final Sod sod = dnie.getSod();
        sod.validateSignature();
        final LdsSecurityObject securityObject = sod.getLdsSecurityObject();
        final DataGroupHash expected = dg13Hash(securityObject);
        final byte[] actual = crypto.digest(
            DigestAlgorithm.getDigestAlgorithm(securityObject.getDigestAlgorithm()),
            dg13Bytes
        );
        if (!Arrays.equals(actual, expected.getDataGroupHashValue())) {
            throw new DniReadException(
                "INTEGRITY_ERROR",
                "DG13 integrity verification failed",
                false,
                null
            );
        }
        return new IntegrityResult("verified", "verified");
    }

    private static DataGroupHash dg13Hash(final LdsSecurityObject securityObject)
        throws DniReadException {
        for (final DataGroupHash hash : securityObject.getDataGroupHashes()) {
            if (hash.getDataGroupNumber() == 13) {
                return hash;
            }
        }
        throw new DniReadException(
            "INTEGRITY_ERROR",
            "SOD does not contain a DG13 hash",
            false,
            null
        );
    }

    private static String readDnieVersion(final Dnie3 dnie) throws Exception {
        return clean(
            new String(
                dnie.selectFileByLocationAndRead(new Location("3F002F03")),
                StandardCharsets.UTF_8
            )
        );
    }

    private static String normalizeDocumentNumber(final String value) {
        return clean(value).replaceAll("[^A-Za-z0-9]", "");
    }

    private static String join(final String first, final String second) {
        return clean(first + " " + second).replaceAll("\\s+", " ");
    }

    private static String clean(final String value) {
        return value == null ? "" : value.trim();
    }

    static String optionalText(final TextSupplier source) {
        try {
            return clean(source.get());
        }
        catch (final Exception e) {
            return "";
        }
    }

    @FunctionalInterface
    interface TextSupplier {
        String get() throws Exception;
    }

    private static void closeQuietly(final SmartcardIoConnection connection) {
        try {
            if (connection.isOpen()) {
                connection.close();
            }
        }
        catch (final Exception ignored) {
            // The worker process ends immediately, so no connection is reused.
        }
    }
}
