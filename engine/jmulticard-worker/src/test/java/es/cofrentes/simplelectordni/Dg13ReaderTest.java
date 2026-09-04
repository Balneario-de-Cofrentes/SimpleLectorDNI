package es.cofrentes.simplelectordni;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.security.SignatureException;
import java.security.cert.CertificateException;

import es.gob.jmulticard.asn1.icao.OptionalDetails;
import es.gob.jmulticard.card.InvalidCardException;
import es.gob.jmulticard.card.icao.Gender;
import es.gob.jmulticard.connection.CardNotPresentException;
import org.junit.jupiter.api.Test;

final class Dg13ReaderTest {

    @Test
    void mapsDocumentFieldsFromDg13WithoutHardware() throws Exception {
        final DocumentData document = Dg13Reader.mapDocument(
            details(Dg13Fixtures.SYNTHETIC_FIELDS),
            () -> " 4.0 ",
            () -> "01020304"
        );

        assertEquals("ANA", document.nombre);
        assertEquals("EJEMPLO", document.primer_apellido);
        assertEquals("PRUEBA", document.segundo_apellido);
        assertEquals("EJEMPLO PRUEBA", document.apellidos);
        assertEquals("00000000-T", document.dni_formateado);
        assertEquals("00000000T", document.dni);
        assertEquals("1990-01-01", document.fecha_nacimiento);
        assertEquals("ESP", document.nacionalidad);
        assertEquals("2030-01-01", document.fecha_caducidad);
        assertEquals("AAA000000", document.numero_soporte);
        assertEquals("F", document.sexo);
        assertEquals("MADRID", document.ciudad_nacimiento);
        assertEquals("PERSONA UNO / PERSONA DOS", document.nombres_progenitores);
        assertEquals("CALLE DE EJEMPLO 1", document.direccion);
        assertEquals("4.0", document.version_dnie);
        assertEquals("01020304", document.serial_chip);
    }

    @Test
    void sexUsesSingleLetterWireValues() {
        assertEquals("M", Dg13Reader.mapSex(Gender.getGender("m")));
        assertEquals("F", Dg13Reader.mapSex(Gender.getGender("F")));
        assertEquals("", Dg13Reader.mapSex(Gender.OTHER));
        assertEquals("", Dg13Reader.mapSex(null));
    }

    @Test
    void optionalEnrichmentFailureDoesNotDiscardDg13() throws Exception {
        final DocumentData document = Dg13Reader.mapDocument(
            details(Dg13Fixtures.SYNTHETIC_FIELDS),
            () -> { throw new IllegalStateException("optional file unavailable"); },
            () -> { throw new IllegalStateException("optional file unavailable"); }
        );

        assertEquals("", document.version_dnie);
        assertEquals("", document.serial_chip);
        assertEquals("ANA", document.nombre);
    }

    @Test
    void refusesADg13WhoseHeaderShiftsJmulticardIndices() throws Exception {
        final String[] fields = Dg13Fixtures.SYNTHETIC_FIELDS.clone();
        fields[fields.length - 1] = Dg13Fixtures.paddingForTotalLength(fields, 0x0102);
        final OptionalDetails details = details(fields);

        final DniReadException failure = assertThrows(
            DniReadException.class,
            () -> Dg13Reader.mapDocument(details, () -> "", () -> "")
        );

        assertEquals(DniErrorCode.DG13_LAYOUT, failure.error());
    }

    @Test
    void deterministicFailuresAreNotRetried() {
        assertEquals(DniErrorCode.UNSUPPORTED_CARD, Dg13Reader.classify(new InvalidCardException("bank card")));
        assertEquals(DniErrorCode.CARD_REMOVED, Dg13Reader.classify(new CardNotPresentException(null)));
        assertEquals(DniErrorCode.INTEGRITY_ERROR, Dg13Reader.classify(new SignatureException("bad")));
        assertEquals(DniErrorCode.INTEGRITY_ERROR, Dg13Reader.classify(new CertificateException("expired")));
        assertEquals(DniErrorCode.CARD_READ_FAILED, Dg13Reader.classify(new IllegalStateException("apdu")));
    }

    private static OptionalDetails details(final String... fields) throws Exception {
        final OptionalDetails details = new OptionalDetails();
        details.setDerValue(Dg13Fixtures.encodedDg13(fields));
        return details;
    }
}
