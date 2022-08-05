import React, {useEffect, useState} from "react";
import Head from "next/head";
import {Button, Card, Divider, Grid, Group, Paper, Radio, Space, Stack, Text, Title} from "@mantine/core";
import {Calendar} from '@mantine/dates';
import {useScrollIntoView} from "@mantine/hooks";
import {NextLink} from "@mantine/next";
import ReservationComponent from "../components/Reservation";
import {LocationName} from "../model/Room";
import 'dayjs/locale/ro'
import {Tuple} from "@mantine/styles/lib/theme/types/Tuple";
import {GameTable, getEndDateDuration, getStartDate, Reservation, ReservationStatus} from "../types/wrapper";
import {supabase} from "../utils/supabase_utils";
import {useAuth} from "../components/AuthProvider";

function addDays(date, days) {
    const result = new Date(date);
    result.setDate(result.getDate() + days);
    return result;
}

interface IParams {
    gara: Room
    boromir: Room
    daysAhead: number
}

interface Room {
    locationName: LocationName
    tables: GameTable[]
    startHour: number
    endHour: number
    duration: number
    maxReservations: number
    uiColor: Tuple<string, 10>
}

class SelectedTable {
    readonly startHour: number
    readonly table: GameTable

    constructor(startHour: number, table: GameTable) {
        this.startHour = startHour
        this.table = table
    }
}

export default function MakeReservationPage(params: IParams): JSX.Element {
    const minRange = addDays(new Date, 1)
    const maxRange = addDays(minRange, params.daysAhead)

    const auth = useAuth()
    const [locationName, setLocationName] = useState(LocationName.Gara)
    const [selectedDate, setSelectedDate] = useState<Date>(null)
    const [selectedTable, setSelectedTable] = useState<SelectedTable>(null)

    function onSelectedDateChange(selectedDate: Date) {
        setSelectedDate(selectedDate)
        setSelectedTable(null)
    }

    const room = locationName == LocationName.Gara ? params.gara : params.boromir;

    return (<>
        <Head>
            <title>Rezervări</title>
            <meta name="viewport" content="initial-scale=1.0, width=device-width"/>
        </Head>

        <Title>Rezervări</Title>

        <Space h="lg"/>

        <Paper>
            {(!auth.loading && auth.user == null) &&
                <>
                    <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                        backgroundColor: theme.colors.orange,
                        marginTop: theme.spacing.sm
                    })}>
                        <Text>Trebuie să ai un cont și să fi un membru pentru a putea face rezervări!</Text>
                    </Paper>
                    <Space h="md"/>
                </>
            }

            <Grid columns={12} gutter={"xl"}>
                <Grid.Col md={6} lg={6} xl={4} key={"calendar"}>
                    <Stack>
                        <Radio.Group
                            value={locationName}
                            onChange={(value) => {
                                switch (value) {
                                    case LocationName.Gara.toString():
                                        setLocationName(LocationName.Gara);
                                        break;
                                    case LocationName.Boromir.toString():
                                        setLocationName(LocationName.Boromir);
                                        break;
                                }
                            }}
                            label={"Alege locația unde faci rezervarea:"}
                            size="md">
                            <Radio value={LocationName.Gara} label={"Gară"}/>
                            <Radio value={LocationName.Boromir} label={"Boromir"}/>
                        </Radio.Group>

                        <Space h={"sm"}/>

                        <Text>Alege ziua rezervării:</Text>

                        <Calendar
                            minDate={minRange}
                            maxDate={maxRange}
                            hideOutsideDates={true}
                            size={"xl"}
                            locale={"ro"}
                            value={selectedDate}
                            onChange={(date) => {
                                if (auth.user != null && date != null) onSelectedDateChange(date)
                            }}
                            excludeDate={(date) => date.getDay() === 0}
                            fullWidth={true}
                        />
                    </Stack>
                </Grid.Col>

                <Grid.Col md={6} lg={6} xl={3} key={"tables"}>
                    {SelectGameTable(room, selectedDate, selectedTable, setSelectedTable)}
                </Grid.Col>

                <Grid.Col md={12} lg={12} xl={5} key={"confirm"}>
                    {ConfirmSelection(room, selectedDate, selectedTable)}
                </Grid.Col>
            </Grid>
        </Paper>
    </>);
}

function SelectGameTable(room: Room,
                         selectedDate: Date,
                         selectedTable: SelectedTable,
                         onSelectTable: (s: SelectedTable) => void): JSX.Element {

    if (selectedDate == null) return null

    const selectedTableId = selectedTable?.table?.id;
    const selectedStartHour = (selectedTable) ? selectedTable.startHour : -1;

    const tableButtons = function (startHour: number) {
        return room.tables.map((gameTable) => {
            return (
                <Button
                    variant={(gameTable.id == selectedTableId && startHour == selectedStartHour) ? "filled" : "outline"}
                    key={gameTable.id}
                    fullWidth={true}
                    onClick={() => onSelectTable(new SelectedTable(startHour, gameTable))}>
                    {gameTable.name}
                </Button>
            )
        })
    }

    const allButtons = () => {
        let content = [];
        for (let startHour = room.startHour; startHour < room.endHour; startHour += room.duration) {
            content.push(<Stack key={startHour}>
                <Text>{`Ora ${startHour} - ${startHour + room.duration}`}:</Text>
                <Group noWrap={true} style={{marginLeft: "1em", marginRight: "1em"}}>
                    {tableButtons(startHour)}
                </Group>
                <Divider variant={"dashed"}/>
            </Stack>);
        }
        return content;
    };

    return (<Stack>
        <Text weight={600}>Data selectată: <Text color={"blue"}>{selectedDate.toLocaleDateString('ro-RO')}</Text></Text>

        {allButtons()}
    </Stack>)
}

function ConfirmSelection(
    room: Room,
    selectedDate: Date, selectedTable: SelectedTable
): JSX.Element {
    const enum ConfirmationStatus {
        None,
        Loading,
        Success,
        Fail
    }

    const auth = useAuth()
    const [status, setStatus] = useState(ConfirmationStatus.None)
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    const [reservations, setReservations] = useState<Reservation[]>([])
    const [currentSelectionReservations, setCurrentSelectionReservations] = useState<Reservation[]>([])

    useEffect(() => {
        function refetchReservations() {
            supabase.from<Reservation>('rezervari')
                .select('*')
                .then(value => {
                    if (value.data != null) {
                        setReservations(value.data)
                        console.log("Reservations updated")
                    }
                })
        }

        refetchReservations();

        const subscription = supabase.from<Reservation>('rezervari')
            .on('*', payload => {
                console.log(JSON.stringify(payload))
                refetchReservations() // TODO Could make this more efficient
            })
            .subscribe()

        return () => {
            subscription?.unsubscribe()
        }
    }, [])

    useEffect(() => {
        if (selectedDate != null && selectedTable != null) {
            setCurrentSelectionReservations(
                reservations.filter((reservation) => {
                        return getStartDate(reservation) == selectedDate
                            && reservation.table_id == selectedTable.table.id
                            && selectedTable.table.location == room.locationName
                    }
                )
            )
        }
    }, [reservations, room.locationName, selectedDate, selectedTable]);

    useEffect(() => {
        if (selectedDate && selectedTable) {
            scrollIntoView({alignment: 'center'})
            setStatus(ConfirmationStatus.None)
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedDate, selectedTable])

    if (selectedDate == null || selectedTable == null) return null

    const startDate = getEndDateDuration(selectedDate, selectedTable.startHour)
    const isValid = currentSelectionReservations.length < room.maxReservations;
    const baseReservation: Reservation = {
        id: self.crypto.randomUUID(),
        start_date: startDate.toISOString(),
        duration: room.duration,
        table_id: selectedTable.table.id,
        user_id: auth.user.id,
    }

    function DisplayConfirmationStatus(): JSX.Element {
        if (status == ConfirmationStatus.None || status == ConfirmationStatus.Loading) {
            return <Button
                fullWidth style={{marginTop: 14}}
                loading={status == ConfirmationStatus.Loading}
                onClick={async () => {
                    setStatus(ConfirmationStatus.Loading)
                    const success = await publishReservation(baseReservation)
                    setStatus(success ? ConfirmationStatus.Success : ConfirmationStatus.Fail)
                }}>Confirmă rezervarea!</Button>
        } else if (status == ConfirmationStatus.Success) {
            return <Stack>
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.green,
                    marginTop: theme.spacing.sm,
                    marginBottom: theme.spacing.xs
                })}>
                    <Text align={"center"} color="#FFF"><b>Cererea ta de rezervare a fost înregistrată</b></Text>
                </Paper>

                <Group align={"center"}>
                    <Text weight={600}>Verifică pe pagina ta de profil dacă rezervarea a fost aprobată:</Text>
                    <NextLink href={"/profile"}>
                        <Button variant={'light'}>Vezi profilul</Button>
                    </NextLink>
                </Group>
            </Stack>
        } else {
            return <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                backgroundColor: theme.colors.orange,
                marginTop: theme.spacing.sm,
                marginBottom: theme.spacing.xs
            })}>
                <Text align={"center"} color="#FFF">Rezervarea nu a putut fi realizată.</Text>
            </Paper>
        }
    }

    return (<Card p={"xl"} shadow={"sm"}>
        <div style={{marginTop: 'sm'}}>
            <Title ref={targetRef}>Confirmă rezervarea:</Title>
        </div>

        <Space h={"lg"}/>

        {ReservationComponent(baseReservation, selectedTable.table, false)}

        <Space h={"md"}/>

        {isValid &&
            DisplayConfirmationStatus()
        }

        {!isValid &&
            <Text>Nu se mai pot face rezervări la aceasta masă</Text>
        }

        {/* { reservations.map(async (reservation) => {
            // const user = appwrite.database.
        })}*/}

    </Card>)
}

async function publishReservation(reservation: Reservation): Promise<boolean> {
    console.log(reservation)

    const {error} = await supabase
        .from<Reservation>('rezervari')
        .insert([reservation])

    if (error != null) {
        console.log("Failed to create reservation: " + JSON.stringify(error))
    }

    return error == null
}

export async function getStaticProps({}) {
    const {data: gameTables} = await supabase.from<GameTable>('mese').select('*')
    const garaTables = gameTables.filter((value) => value.location == LocationName.Gara)
    const boromirTables = gameTables.filter((value) => value.location == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: {
            locationName: LocationName.Gara,
            tables: garaTables,
            startHour: 18,
            endHour: 22,
            duration: 2,
            maxReservations: 8,
            uiColor: ['#e7f5ff', '#d0ebff', '#a5d8ff', '#74c0fc', '#4dabf7', '#339af0', '#228be6', '#1c7ed6', '#1971c2', '#1864ab']
        },
        boromir: {
            locationName: LocationName.Boromir,
            tables: boromirTables,
            startHour: 10,
            endHour: 20,
            duration: 1,
            maxReservations: 2,
            uiColor: ['#e6fcf5', '#c3fae8', '#96f2d7', '#63e6be', '#38d9a9', '#20c997', '#12b886', '#0ca678', '#099268', '#087f5b',]
        }
    }

    return {
        props: props
    }
}
