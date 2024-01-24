import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import {useEffect, useMemo, useState} from "react";
import {Reservation} from "../../types/wrapper";
import {Button, Card, Divider, Grid, Group, Indicator, Select, Space, Stack, Text} from "@mantine/core";
import {DatePicker} from "@mantine/dates";
import 'dayjs/locale/ro';
import {dateToISOString} from "../../utils/date";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import useProfilesQuery from "../../hooks/useProfilesQuery";

export default function SituationPage() {
    const supabase = useSupabaseClient<Database>()

    const {data: allProfiles} = useProfilesQuery()
    const [reservations, setReservations] = useState<Reservation[]>([])
    const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
    const [dateRange, setDateRange] = useState<[Date | null, Date | null]>([null, null]);
    const [startRange, endRange] = dateRange;

    useExitIfNotFounder();

    useEffect(() => {
        if (selectedProfileId != null) {
            supabase.from('rezervari').select('*')
                .eq('user_id', selectedProfileId)
                .order('start_date', {ascending: false})
                .order('start_hour', {ascending: true})
                .then(value => {
                    if (value.data != null) {
                        setReservations(value.data)
                    }
                })
        }
    }, [selectedProfileId, supabase])

    const filteredReservations = useMemo(
        () => {
            if (startRange == null || endRange == null) {
                return []
            }
            return reservations.filter(it =>
                it.user_id === selectedProfileId &&
                new Date(it.start_date) >= startRange && new Date(it.start_date) <= endRange)
        },
        [reservations, selectedProfileId, startRange, endRange]
    )

    return <Grid
        grow={true}
        columns={4}
        style={{
            marginBottom: `var(--mantine-spacing-xl)`
        }}
    >
        <Grid.Col span={"auto"}>
            <Stack p={'md'}>
                <Select
                    label="Alege un member"
                    searchable
                    data={allProfiles?.map(profile => ({value: profile.id, label: profile.name})) || []}
                    value={selectedProfileId}
                    required={true}
                    error={selectedProfileId == null ? 'Trebuie sa alegi un membru!' : null}
                    onChange={setSelectedProfileId}
                />

                <DatePicker
                    type='range'
                    value={dateRange}
                    onChange={setDateRange}
                    size={"lg"}
                    locale="ro"
                    renderDay={(date) => {
                        const disabled = !filteredReservations.some(it =>
                            it.start_date === dateToISOString(date) && !it.cancelled);

                        return (
                            <Indicator
                                size={8} color="green" offset={-5}
                                disabled={disabled}>
                                {date.getDate()}
                            </Indicator>
                        );
                    }}
                />

                <Divider />

                <Button onClick={() => setDateRange([new Date('2024-01-01'), new Date('2024-12-31')])}>
                    Selectează tot anul 2024
                </Button>

                <Button onClick={() => setDateRange([new Date('2023-01-01'), new Date('2023-12-31')])}>
                    Selectează tot anul 2023
                </Button>
            </Stack>
        </Grid.Col>

        <Grid.Col span={2}>
            <Stack p={'md'} id={"report"} style={{backgroundColor: `var(--mantine-color-dark-7)`}}>
                {selectedProfileId &&
                    SelectedUserReservations(filteredReservations)
                }
            </Stack>
        </Grid.Col>
    </Grid>
}

function SelectedUserReservations(reservations: Reservation[]) {
    const approvedReservations = reservations.filter((it) => !it.cancelled)
    const cancelledReservations = reservations.filter((it) => it.cancelled)

    return <>
        <Text size={'xl'}>Total rezervări: {approvedReservations.length}</Text>

        {
            approvedReservations.map((reservation, index) => {
                return <Card key={reservation.id}>
                    <Group justify="space-between">
                        <Text>{index + 1}</Text>

                        <Text fw={900}>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</Text>

                        <Text>De
                            la <b>{reservation.start_hour}:{'00'}</b></Text>
                    </Group>
                </Card>
            })
        }

        <Space h={'lg'}/>

        <Text size={'xl'}>Total rezervări anulate: {cancelledReservations.length}</Text>
        {
            cancelledReservations.map((reservation, index) => {
                return <Card key={reservation.id}>
                    <Group justify="space-between">
                        <Text>{index + 1}</Text>

                        <Text fw={900}>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</Text>

                        <Text>De la <b>{reservation.start_hour}:{'00'}</b></Text>
                    </Group>
                </Card>
            })
        }
    </>
}
