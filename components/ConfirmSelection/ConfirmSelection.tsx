import React, {useEffect, useState} from "react";
import {useScrollIntoView} from "@mantine/hooks";
import {Reservation, ReservationStatus} from "../../types/wrapper";
import {Button, Card, Group, Paper, Space, Stack, Text, Title} from "@mantine/core";
import ReservationComponent from "../Reservation";
import {Room, SelectedTable} from "../../types/room";
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
    room: Room,
    selectedDateISO: string | null,
    selectedTable: SelectedTable | null
): JSX.Element {
    const supabase = useSupabaseClient<Database>()

    const [status, setStatus] = useState(ConfirmationStatus.None)
    const [errorMessage, setErrorMessage] = useState("")
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        if (selectedDateISO && selectedTable) {
            scrollIntoView({alignment: 'center'})
            setStatus(ConfirmationStatus.None)
            setErrorMessage("")
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedDateISO, selectedTable])

    if (selectedDateISO == null || selectedTable == null) return <></>

    // Fake Reservation to display to user
    const fakeReservation: Reservation = {
        id: '',
        created_at: '',
        user_id: '',
        table_id: selectedTable.table.id,
        start_date: selectedDateISO,
        start_hour: selectedTable.startHour,
        duration: room.duration,
        status: ReservationStatus.Approved
    }

    function DisplayConfirmationStatus(): JSX.Element {
        if (status == ConfirmationStatus.None || status == ConfirmationStatus.Loading) {
            return <Button
                fullWidth style={{marginTop: 14}}
                loading={status == ConfirmationStatus.Loading}
                color={'blue'}
                onClick={async () => {
                    setStatus(ConfirmationStatus.Loading)
                    const reservationParams = {
                        table_id_input: selectedTable!.table.id,
                        start_date_input: selectedDateISO,
                        start_hour_input: selectedTable!.startHour,
                    }
                    const errorMessage = await publishReservation(supabase, reservationParams)
                    setStatus(errorMessage == null ? ConfirmationStatus.Success : ConfirmationStatus.Fail)
                    setErrorMessage(errorMessage ?? "")
                }}>Confirmă rezervarea!</Button>
        } else if (status == ConfirmationStatus.Success) {
            return <Stack>
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.green,
                    marginTop: theme.spacing.sm,
                    marginBottom: theme.spacing.xs
                })}>
                    <Text align={"center"} color="#FFF"><b>Rezervarea ta a fost înregistrată</b></Text>
                </Paper>

                <Group align={"center"}>
                    <Text weight={600}>Rezervarea poate fi anulată de pe pagina ta de profil:</Text>
                    <Link href={"/profile"}>
                        <Button variant={'light'}>Vezi profilul</Button>
                    </Link>
                </Group>
            </Stack>
        } else {
            return <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                backgroundColor: theme.colors.orange,
                marginTop: theme.spacing.sm,
                marginBottom: theme.spacing.xs
            })}>
                <Text align={"center"} color="#FFF">Rezervarea nu a putut fi realizată: {errorMessage}</Text>
            </Paper>
        }
    }

    return (<Card p={"xl"} shadow={"sm"}>
        <div style={{marginTop: 'sm'}}>
            <Title order={2} ref={targetRef}>Confirmă rezervarea:</Title>
        </div>

        <Space h={"lg"}/>

        {ReservationComponent(fakeReservation, selectedTable.table, false, null)}

        <Space h={"md"}/>

        <DisplayConfirmationStatus/>

    </Card>)
}

async function publishReservation(supabase: SupabaseClient<Database>, reservationParams): Promise<string | null> {
    console.log(reservationParams)

    const {data, error} = await supabase.rpc('create_reservation', reservationParams)

    if (error != null) return "a fost întâmpinată o eroare (" + error.message + ")"

    // @ts-ignore TypeScript is actually wrong here
    return data == '' ? null : data // An empty string means no errors
}
