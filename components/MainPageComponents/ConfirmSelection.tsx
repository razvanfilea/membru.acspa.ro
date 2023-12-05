import React, {ReactElement, useEffect, useState} from "react";
import {useScrollIntoView} from "@mantine/hooks";
import {Location, Reservation} from "../../types/wrapper";
import {Button, Card, Group, Paper, Space, Stack, Text, Title} from "@mantine/core";
import ReservationComponent from "../Reservation";
import Link from "next/link";
import {SupabaseClient, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";

const enum ConfirmationStatus {
    None,
    Loading,
    Success,
    Fail
}

export function ConfirmSelection(
    location: Location,
    selectedDateISO: string | null,
    selectedStartHour: number | null
): ReactElement {
    const supabase = useSupabaseClient<Database>()

    const [status, setStatus] = useState(ConfirmationStatus.None)
    const [responseMessage, setResponseMessage] = useState<string | null>(null)
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        if (selectedDateISO && selectedStartHour) {
            scrollIntoView({alignment: 'center'})
            setStatus(ConfirmationStatus.None)
            setResponseMessage("")
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedDateISO, selectedStartHour])

    if (selectedDateISO == null || selectedStartHour == null) return <></>

    // Fake Reservation to display to user
    const fakeReservation: Reservation = {
        id: '',
        created_at: '',
        user_id: '',
        location: location.name,
        start_date: selectedDateISO,
        start_hour: selectedStartHour,
        cancelled: false
    }

    function DisplayConfirmationStatus(): ReactElement {
        if (status == ConfirmationStatus.None || status == ConfirmationStatus.Loading) {
            return <Button
                fullWidth style={{marginTop: 14}}
                loading={status == ConfirmationStatus.Loading}
                color={'blue'}
                onClick={async () => {
                    setStatus(ConfirmationStatus.Loading)
                    const reservationParams: ReservationParams = {
                        location_input: location.name,
                        start_date_input: selectedDateISO!,
                        start_hour_input: selectedStartHour!,
                    }

                    const reservationResult = await publishReservation(supabase, reservationParams)

                    setStatus(reservationResult.success ? ConfirmationStatus.Success : ConfirmationStatus.Fail)
                    setResponseMessage(reservationResult.message)
                }}>Confirmă rezervarea!</Button>
        } else if (status == ConfirmationStatus.Success) {
            return <></>
        } else {
            return <Paper shadow={"0"} p={"md"} style={{
                backgroundColor: `var(--mantine-color-orange)`,
                marginTop: `var(--mantine-spacing-sm)`,
                marginBottom: `var(--mantine-spacing-xs)`
            }}>
                <Text ta={"center"} c="#FFF">Rezervarea nu a putut fi realizată: {responseMessage}</Text>
            </Paper>
        }
    }

    return <Card p={"xl"} shadow={"sm"}>
        {status !== ConfirmationStatus.Success &&
            <>
                <div style={{marginTop: 'sm'}}>
                    <Title order={2} ref={targetRef}>Confirmă rezervarea:</Title>
                </div>

                <Space h={"lg"}/>

                {ReservationComponent(fakeReservation, location, false, null)}

                <Space h={"md"}/>

                <DisplayConfirmationStatus/>
            </>
        }

        {status === ConfirmationStatus.Success &&
            <Stack>
                <Paper shadow={"0"} px={';g'} py={"md"} withBorder={true} radius={'xl'} style={{
                    color: responseMessage == null ? `var(--mantine-color-green)` : `var(--mantine-color-blue)`,
                    marginTop: `var(--mantine-spacing-sm)`,
                    marginBottom: `var(--mantine-spacing-xs)`
                }}>
                    <Text ta={"center"} c="#FFF">
                        {responseMessage == null ?
                            <>Ai rezervare
                                pe <b>{(new Date(fakeReservation.start_date)).toLocaleDateString('ro-RO')}</b> de la
                                ora <b>{fakeReservation.start_hour}:{'00'}</b> la <b>{fakeReservation.start_hour + location.reservation_duration}:{'00'}</b>
                            </>
                            : responseMessage
                        }
                    </Text>
                </Paper>

                {responseMessage == null &&
                    <Group align={"center"}>
                        <Text fw={600}>Rezervarea poate fi anulată de pe pagina ta de profil:</Text>
                        <Link href={"/profile"}>
                            <Button variant={'light'}>Vezi profilul</Button>
                        </Link>
                    </Group>
                }
            </Stack>
        }

    </Card>
}

interface ReservationParams {
    location_input: string;
    start_date_input: string;
    start_hour_input: number;
}

interface ReservationResult {
    success: boolean;
    message: string | null;
}

async function publishReservation(
    supabase: SupabaseClient<Database>,
    reservationParams: ReservationParams
): Promise<ReservationResult> {

    const {data, error} = await supabase.rpc('create_reservation', reservationParams)

    return {
        success: data != null,
        message: data || error?.message || null
    }
}
