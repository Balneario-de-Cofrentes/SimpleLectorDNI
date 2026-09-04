package es.cofrentes.simplelectordni;

import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.Arrays;
import java.util.Date;

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
    public DniReadResult read(final int readerIndex) throws DniReadException {
        final SmartcardIoConnection connection = new SmartcardIoConnection();
        try {
            return readConnected(connection, readerIndex);
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
        final int readerIndex
    ) throws Exception {
        connection.setProtocol(ApduConnectionProtocol.T0);
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
    ) throws Exception {
        final DocumentData value = new DocumentData();
        value.nombre = clean(details.getName());
        value.primer_apellido = clean(details.getFirstSurname());
        value.segundo_apellido = clean(details.getSecondSurname());
        value.apellidos = join(value.primer_apellido, value.segundo_apellido);
        value.dni_formateado = clean(details.getIdNumber());
        value.dni = normalizeDocumentNumber(value.dni_formateado);
        value.fecha_nacimiento = formatDate(details.getBirthDate());
        value.nacionalidad = clean(details.getNationality());
        value.fecha_caducidad = formatDate(details.getExpirationDate());
        mapAdditionalDetails(value, details);
        value.version_dnie = readDnieVersion(dnie);
        value.serial_chip = HexUtils.hexify(dnie.getSerialNumber(), false);
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

    private static String formatDate(final Date value) {
        return value == null ? "" : new SimpleDateFormat("yyyy-MM-dd").format(value);
    }

    private static String join(final String first, final String second) {
        return clean(first + " " + second).replaceAll("\\s+", " ");
    }

    private static String clean(final String value) {
        return value == null ? "" : value.trim();
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
